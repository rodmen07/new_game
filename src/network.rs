//! Multiplayer position sync via a WebSocket relay server.
//!
//! Architecture (WASM only):
//! - On `OnEnter(Playing)` we open a WebSocket to the relay.
//! - Every `NET_SEND_INTERVAL` seconds we send our local player position.
//! - Incoming messages are buffered via Rc<RefCell<>> callbacks and drained
//!   each frame in `net_receive`.
//! - Remote players are spawned/despawned/moved as `Player` entities
//!   (without `LocalPlayer`) so all existing queries continue to work.

#[cfg(target_arch = "wasm32")]
use bevy::prelude::*;

// ── Only compile the real networking on WASM ──────────────────────────────────

#[cfg(target_arch = "wasm32")]
pub mod wasm_net {
    use super::*;
    use crate::components::{Player, RemotePlayer};
    use js_sys::JsString;
    use serde::{Deserialize, Serialize};
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};
    use wasm_bindgen::{JsCast, closure::Closure};
    use web_sys::{ErrorEvent, MessageEvent, WebSocket};

    // ── Config ────────────────────────────────────────────────────────────────

    /// URL of the relay.  In development use `ws://localhost:8090/ws`.
    /// Set RELAY_URL at build time or fall back to the Fly.io deployment.
    const RELAY_URL: &str = match option_env!("RELAY_URL") {
        Some(url) => url,
        None => "wss://multiplayer-relay-rodmen07.fly.dev/ws",
    };

    /// How often (seconds) we send our position to the server.
    const NET_SEND_INTERVAL: f32 = 0.05; // 20 Hz

    /// Exponential interpolation rate used to smooth remote player motion.
    const REMOTE_INTERP_RATE: f32 = 16.0;

    /// If no updates are received for this long, despawn the remote player.
    const REMOTE_STALE_SECS: f32 = 12.0;

    /// Hard cap on concurrent remote players. The relay is untrusted; a
    /// malicious or buggy server could otherwise stream unique ids forever
    /// and exhaust the WASM heap. New ids beyond this are dropped.
    const MAX_REMOTE_PLAYERS: usize = 64;

    /// Maximum accepted length (bytes) for a player id from the server.
    /// Real ids are short uuids/nanoids; anything larger is treated as abuse.
    const MAX_NET_ID_LEN: usize = 64;

    /// Maximum messages drained from the inbox per frame, to bound per-frame
    /// work even if a misbehaving server bursts traffic.
    const MAX_INBOX_DRAIN_PER_FRAME: usize = 256;

    // ── Incoming message types ────────────────────────────────────────────────

    #[derive(Deserialize, Debug)]
    #[serde(tag = "type", rename_all = "lowercase")]
    pub enum ServerMsg {
        Welcome { id: String },
        Pos { id: String, x: f32, y: f32 },
        Leave { id: String },
    }

    #[derive(Serialize)]
    struct PosMsg {
        #[serde(rename = "type")]
        kind: &'static str,
        x: f32,
        y: f32,
    }

    // ── Resource ──────────────────────────────────────────────────────────────

    pub struct NetState {
        pub socket: WebSocket,
        pub local_id: Option<String>,
        pub inbox: Rc<RefCell<VecDeque<ServerMsg>>>,
        pub send_timer: f32,
    }

    #[derive(Component)]
    pub(crate) struct RemoteNetSmoothing {
        target_position: Vec2,
        seconds_since_update: f32,
    }

    // ── Plugin ────────────────────────────────────────────────────────────────

    pub struct MultiplayerPlugin;

    impl Plugin for MultiplayerPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(OnEnter(crate::menu::AppState::Playing), net_connect)
                .add_systems(OnExit(crate::menu::AppState::Playing), net_disconnect)
                .add_systems(
                    Update,
                    (
                        net_send,
                        net_receive,
                        net_smooth_remote_players,
                        net_prune_stale_remote_players,
                    )
                        .chain()
                        .run_if(bevy::prelude::in_state(crate::menu::AppState::Playing)),
                );
        }
    }

    // ── Systems ───────────────────────────────────────────────────────────────

    pub fn net_connect(world: &mut World) {
        let inbox: Rc<RefCell<VecDeque<ServerMsg>>> = Rc::new(RefCell::new(VecDeque::new()));

        let ws = match WebSocket::new(RELAY_URL) {
            Ok(ws) => ws,
            Err(e) => {
                bevy::log::warn!("net_connect: WebSocket::new failed: {:?}", e);
                return;
            }
        };

        {
            let inbox_clone = inbox.clone();
            let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                if let Ok(txt) = e.data().dyn_into::<JsString>() {
                    let s = String::from(txt);
                    // Cap parsed payload size; the relay protocol is tiny json.
                    if s.len() > 4096 {
                        bevy::log::warn!("net: dropping oversized message ({} bytes)", s.len());
                        return;
                    }
                    match serde_json::from_str::<ServerMsg>(&s) {
                        Ok(msg) => {
                            // Reject ids that are absurdly large up front so we
                            // don't keep multi-MB strings alive in the inbox.
                            let id_ok = match &msg {
                                ServerMsg::Welcome { id }
                                | ServerMsg::Pos { id, .. }
                                | ServerMsg::Leave { id } => id.len() <= MAX_NET_ID_LEN,
                            };
                            if id_ok {
                                inbox_clone.borrow_mut().push_back(msg);
                            } else {
                                bevy::log::warn!("net: dropping message with oversized id");
                            }
                        }
                        Err(err) => bevy::log::warn!("net: parse error: {err} raw={s}"),
                    }
                }
            });
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();
        }

        {
            let onerror = Closure::<dyn FnMut(ErrorEvent)>::new(|e: ErrorEvent| {
                bevy::log::warn!("net: WebSocket error: {:?}", e.message());
            });
            ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();
        }

        world.insert_non_send_resource(NetState {
            socket: ws,
            local_id: None,
            inbox,
            send_timer: 0.,
        });
    }

    pub fn net_disconnect(world: &mut World) {
        if let Some(state) = world.get_non_send_resource::<NetState>() {
            let _ = state.socket.close();
        }

        let mut q = world.query_filtered::<Entity, With<RemotePlayer>>();
        let remote_entities: Vec<Entity> = q.iter(world).collect();
        for entity in remote_entities {
            let _ = world.despawn(entity);
        }

        let _ = world.remove_non_send_resource::<NetState>();
    }

    pub fn net_send(
        time: Res<Time>,
        net: Option<NonSendMut<NetState>>,
        player_q: Query<&Transform, With<crate::components::LocalPlayer>>,
    ) {
        let Some(mut net) = net else {
            return;
        };

        net.send_timer += time.delta_secs();
        if net.send_timer < NET_SEND_INTERVAL {
            return;
        }
        net.send_timer = 0.;

        let Some(tf) = player_q.iter().next() else {
            return;
        };

        let msg = serde_json::to_string(&PosMsg {
            kind: "pos",
            x: tf.translation.x,
            y: tf.translation.y,
        })
        .unwrap_or_default();

        if net.socket.ready_state() == WebSocket::OPEN {
            let _ = net.socket.send_with_str(&msg);
        }
    }

    pub fn net_receive(
        mut commands: Commands,
        net: Option<NonSendMut<NetState>>,
        mut remote_q: Query<(
            Entity,
            &RemotePlayer,
            &mut Transform,
            &mut RemoteNetSmoothing,
        )>,
    ) {
        let Some(mut net) = net else {
            return;
        };

        // Drain at most a bounded number of messages per frame; leave the
        // rest in the inbox so a flood from a misbehaving relay can't stall
        // the main thread.
        let messages: Vec<ServerMsg> = {
            let mut inbox = net.inbox.borrow_mut();
            let take = inbox.len().min(MAX_INBOX_DRAIN_PER_FRAME);
            inbox.drain(..take).collect()
        };

        for msg in messages {
            match msg {
                ServerMsg::Welcome { id } => {
                    // Only honor the first Welcome. A malicious relay must
                    // not be able to rename us mid-session, which would
                    // bypass the self-position filter below.
                    if net.local_id.is_some() {
                        bevy::log::warn!("net: ignoring duplicate Welcome");
                        continue;
                    }
                    bevy::log::info!("net: connected as {id}");
                    net.local_id = Some(id);
                }
                ServerMsg::Pos { id, x, y } => {
                    if net.local_id.as_deref() == Some(&id) {
                        continue;
                    }
                    // Reject NaN/Inf positions outright.
                    if !x.is_finite() || !y.is_finite() {
                        continue;
                    }

                    let mut found_existing = false;
                    let mut remote_count: usize = 0;
                    for (_, rp, mut tf, mut smoothing) in &mut remote_q {
                        remote_count += 1;
                        if rp.net_id != id {
                            continue;
                        }

                        let target = Vec2::new(x, y);
                        smoothing.target_position = target;
                        smoothing.seconds_since_update = 0.0;

                        // Snap if we fell too far behind (late packet burst/reconnect).
                        if tf.translation.truncate().distance_squared(target) > 2500.0 {
                            tf.translation.x = x;
                            tf.translation.y = y;
                        }

                        found_existing = true;
                        break;
                    }

                    if !found_existing {
                        if remote_count >= MAX_REMOTE_PLAYERS {
                            // Don't let an attacker exhaust the heap by
                            // streaming unique ids forever.
                            continue;
                        }
                        spawn_remote(&mut commands, id, x, y);
                    }
                }
                ServerMsg::Leave { id } => {
                    for (entity, rp, _, _) in &remote_q {
                        if rp.net_id == id {
                            commands.entity(entity).despawn_recursive();
                            bevy::log::info!("net: player {id} left");
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn net_smooth_remote_players(
        time: Res<Time>,
        mut remote_q: Query<(&mut Transform, &RemoteNetSmoothing), With<RemotePlayer>>,
    ) {
        let dt = time.delta_secs().max(0.0);
        let t = (1.0 - (-REMOTE_INTERP_RATE * dt).exp()).clamp(0.0, 1.0);

        for (mut tf, smoothing) in &mut remote_q {
            let current = tf.translation.truncate();
            let next = current.lerp(smoothing.target_position, t);
            tf.translation.x = next.x;
            tf.translation.y = next.y;
        }
    }

    pub fn net_prune_stale_remote_players(
        mut commands: Commands,
        time: Res<Time>,
        mut remote_q: Query<(Entity, &RemotePlayer, &mut RemoteNetSmoothing)>,
    ) {
        let dt = time.delta_secs();
        for (entity, rp, mut smoothing) in &mut remote_q {
            smoothing.seconds_since_update += dt;
            if smoothing.seconds_since_update > REMOTE_STALE_SECS {
                bevy::log::info!("net: pruning stale remote player {}", rp.net_id);
                commands.entity(entity).despawn_recursive();
            }
        }
    }

    fn spawn_remote(commands: &mut Commands, id: String, x: f32, y: f32) {
        bevy::log::info!("net: spawning remote player {id}");
        commands.spawn((
            Player,
            RemotePlayer { net_id: id },
            RemoteNetSmoothing {
                target_position: Vec2::new(x, y),
                seconds_since_update: 0.0,
            },
            Sprite {
                color: Color::srgb(0.35, 0.60, 1.0), // blue tint for remote players
                custom_size: Some(Vec2::new(20., 32.)),
                ..default()
            },
            Transform::from_xyz(x, y, 10.0),
            Visibility::default(),
        ));
    }
}

// ── Stub plugin for non-WASM builds (tests, native dev) ──────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub mod wasm_net {
    use bevy::prelude::*;

    pub struct MultiplayerPlugin;

    impl Plugin for MultiplayerPlugin {
        fn build(&self, _app: &mut App) {
            // no-op on native
        }
    }
}
