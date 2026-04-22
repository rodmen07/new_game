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

    // ── Plugin ────────────────────────────────────────────────────────────────

    pub struct MultiplayerPlugin;

    impl Plugin for MultiplayerPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(OnEnter(crate::menu::AppState::Playing), net_connect)
                .add_systems(OnExit(crate::menu::AppState::Playing), net_disconnect)
                .add_systems(
                    Update,
                    (net_send, net_receive)
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
                    match serde_json::from_str::<ServerMsg>(&s) {
                        Ok(msg) => inbox_clone.borrow_mut().push_back(msg),
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
        mut remote_q: Query<(Entity, &RemotePlayer, &mut Transform)>,
    ) {
        let Some(mut net) = net else {
            return;
        };

        let messages: Vec<ServerMsg> = net.inbox.borrow_mut().drain(..).collect();

        for msg in messages {
            match msg {
                ServerMsg::Welcome { id } => {
                    bevy::log::info!("net: connected as {id}");
                    net.local_id = Some(id);
                }
                ServerMsg::Pos { id, x, y } => {
                    if net.local_id.as_deref() == Some(&id) {
                        continue;
                    }

                    let existing = remote_q.iter_mut().find(|(_, rp, _)| rp.net_id == id).map(
                        |(e, _, mut tf)| {
                            tf.translation.x = x;
                            tf.translation.y = y;
                            e
                        },
                    );

                    if existing.is_none() {
                        spawn_remote(&mut commands, id, x, y);
                    }
                }
                ServerMsg::Leave { id } => {
                    for (entity, rp, _) in &remote_q {
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

    fn spawn_remote(commands: &mut Commands, id: String, x: f32, y: f32) {
        bevy::log::info!("net: spawning remote player {id}");
        commands.spawn((
            Player,
            RemotePlayer { net_id: id },
            Sprite {
                color: Color::srgb(0.35, 0.60, 1.0), // blue tint for remote players
                custom_size: Some(Vec2::new(20., 32.)),
                ..default()
            },
            Transform::from_xyz(x, y, 1.0),
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
