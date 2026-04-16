use bevy::prelude::*;
use bevy::input::mouse::{MouseWheel, MouseScrollUnit};
use crate::constants::PLAYER_SPEED;
use crate::components::{Player, MainCamera, PlayerIndicator, BodyPart};
use crate::resources::{PlayerStats, PlayerMovement, Transport, TransportKind};

const ACCEL:        f32 = 1400.;  // pixels/s² acceleration
const FRICTION:     f32 = 900.;   // pixels/s² deceleration when no input
const SPRINT_MULT:  f32 = 1.75;   // sprint speed multiplier
const SPRINT_DRAIN: f32 = 3.5;    // energy drained per second while sprinting
const DASH_SPEED:   f32 = 620.;   // pixels/s during dash
const DASH_DURATION:f32 = 0.18;   // seconds the dash lasts
const DASH_COOLDOWN:f32 = 1.20;   // seconds before next dash
const DASH_COST:    f32 = 4.;     // energy cost per dash

pub fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut stats: ResMut<PlayerStats>,
    transport: Res<Transport>,
    mut pm: ResMut<PlayerMovement>,
    mut q: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut tf) = q.get_single_mut() else { return };
    let dt = time.delta_secs();

    // ── Dash cooldown tick ─────────────────────────────────────────────────
    if pm.dash_cooldown > 0. { pm.dash_cooldown = (pm.dash_cooldown - dt).max(0.); }

    // ── Dash active tick ───────────────────────────────────────────────────
    if pm.dashing {
        pm.dash_timer -= dt;
        if pm.dash_timer <= 0. {
            pm.dashing = false;
            pm.velocity = pm.dash_dir * PLAYER_SPEED * 0.5; // exit dash with carry-over velocity
        } else {
            let mv = pm.dash_dir * DASH_SPEED * dt;
            tf.translation.x = (tf.translation.x + mv.x).clamp(-1300., 1300.);
            tf.translation.y = (tf.translation.y + mv.y).clamp(-1300., 1300.);
            return;
        }
    }

    // ── Input direction ────────────────────────────────────────────────────
    let mut wish = Vec2::ZERO;
    if keys.any_pressed([KeyCode::ArrowUp,    KeyCode::KeyW]) { wish.y += 1.; }
    if keys.any_pressed([KeyCode::ArrowDown,  KeyCode::KeyS]) { wish.y -= 1.; }
    if keys.any_pressed([KeyCode::ArrowLeft,  KeyCode::KeyA]) { wish.x -= 1.; }
    if keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) { wish.x += 1.; }
    let sprinting = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
        && wish != Vec2::ZERO && stats.energy > 5.;

    // ── Dash trigger (Space) ───────────────────────────────────────────────
    let dash_dir = if wish != Vec2::ZERO { wish.normalize() } else if pm.velocity.length() > 10. { pm.velocity.normalize() } else { Vec2::Y };
    if keys.just_pressed(KeyCode::Space) && pm.dash_cooldown <= 0. && stats.energy >= DASH_COST {
        stats.energy = (stats.energy - DASH_COST).max(0.);
        pm.dashing   = true;
        pm.dash_timer = DASH_DURATION;
        pm.dash_cooldown = DASH_COOLDOWN;
        pm.dash_dir  = dash_dir;
        return;
    }

    // ── Compute max speed ──────────────────────────────────────────────────
    let health_mult = if stats.health < 30. { 0.4 } else if stats.energy < 20. { 0.6 } else { 1.0 };
    let transport_mult = match transport.kind {
        TransportKind::Bike => 1.15,
        TransportKind::Car  => 1.30,
        TransportKind::Walk => 1.0,
    };
    let sprint_mult = if sprinting { SPRINT_MULT } else { 1.0 };
    let max_speed = PLAYER_SPEED * health_mult * transport_mult * sprint_mult;

    // ── Accelerate / friction ──────────────────────────────────────────────
    if wish != Vec2::ZERO {
        let accel_vec = wish.normalize() * ACCEL * dt;
        pm.velocity += accel_vec;
        // Clamp to max speed (preserve direction)
        if pm.velocity.length() > max_speed {
            pm.velocity = pm.velocity.normalize() * max_speed;
        }
    } else {
        // Friction
        let speed = pm.velocity.length();
        let friction = (FRICTION * dt).min(speed);
        if speed > 0. {
            let dir = pm.velocity / speed;
            pm.velocity -= dir * friction;
        }
    }

    // ── Sprint energy drain ────────────────────────────────────────────────
    if sprinting {
        pm.sprint_energy_timer += dt;
        if pm.sprint_energy_timer >= 0.5 {
            stats.energy = (stats.energy - SPRINT_DRAIN * pm.sprint_energy_timer).max(0.);
            pm.sprint_energy_timer = 0.;
        }
    } else {
        pm.sprint_energy_timer = 0.;
    }

    // ── Apply movement ─────────────────────────────────────────────────────
    if pm.velocity.length() > 0.5 {
        tf.translation.x = (tf.translation.x + pm.velocity.x * dt).clamp(-1300., 1300.);
        tf.translation.y = (tf.translation.y + pm.velocity.y * dt).clamp(-1300., 1300.);
    } else {
        pm.velocity = Vec2::ZERO;
    }
}

/// Squash/stretch via root scale; animate legs; flash body colour by state; orbit direction dot.
pub fn player_visuals(
    pm:    Res<PlayerMovement>,
    stats: Res<PlayerStats>,
    time:  Res<Time>,
    mut player_q:    Query<(&Children, &mut Transform), With<Player>>,
    mut parts_q:     Query<(&BodyPart, &mut Sprite, &mut Transform), (Without<Player>, Without<PlayerIndicator>)>,
    mut indicator_q: Query<&mut Transform, (With<PlayerIndicator>, Without<Player>)>,
) {
    let Ok((children, mut root_tf)) = player_q.get_single_mut() else { return };
    let speed = if pm.dashing { DASH_SPEED } else { pm.velocity.length() };
    let speed_norm = (speed / (PLAYER_SPEED * SPRINT_MULT)).clamp(0., 1.);
    let t = time.elapsed_secs();

    // Squash/stretch via root scale (propagates to all children)
    let (sx, sy) = if pm.dashing {
        (1.25, 0.80)
    } else {
        let bob = if speed > 10. {
            (t * (7. + speed * 0.012)).sin() * speed_norm * 0.06
        } else { 0. };
        (1. - speed_norm * 0.08 - bob * 0.4, 1. + speed_norm * 0.12 + bob)
    };
    root_tf.scale = Vec3::new(sx, sy, 1.);

    // Body colour by state (flashes on dash / critical / low stats)
    let body_color = if stats.critical_timer > 0. {
        let f = (t * 8.).sin() * 0.4 + 0.6;
        Color::srgb(1., f * 0.12, f * 0.04)
    } else if pm.dashing {
        let f = (t * 28.).sin() * 0.3 + 0.7;
        Color::srgb(1.0, 0.80 + f * 0.20, 0.55 + f * 0.35)
    } else if stats.health < 30. {
        Color::srgb(0.82, 0.28, 0.14)
    } else if stats.energy < 20. {
        Color::srgb(0.70, 0.65, 0.48)
    } else {
        Color::srgb(0.90, 0.52, 0.12)
    };

    // Leg walk cycle — opposite phase left/right
    let leg_amp   = if pm.dashing { 0. } else { speed_norm * 3.5 };
    let leg_phase = (t * (7. + speed * 0.015)).sin() * leg_amp;

    // Direction for indicator orbit
    let move_dir  = if pm.velocity.length() > 10. { pm.velocity.normalize() } else { Vec2::Y };
    let orbit_dist = 18.;

    for &child in children.iter() {
        // Body parts — animate legs + recolour torso
        if let Ok((part, mut sprite, mut ctf)) = parts_q.get_mut(child) {
            match *part {
                BodyPart::Body      => { sprite.color = body_color; }
                BodyPart::LeftLeg   => { ctf.translation.y = -5. + leg_phase; }
                BodyPart::RightLeg  => { ctf.translation.y = -5. - leg_phase; }
                BodyPart::LeftFoot  => { ctf.translation.y = -10. + leg_phase * 0.65; }
                BodyPart::RightFoot => { ctf.translation.y = -10. - leg_phase * 0.65; }
                _ => {}
            }
        }
        // Direction indicator
        if let Ok(mut ctf) = indicator_q.get_mut(child) {
            ctf.translation = Vec3::new(move_dir.x * orbit_dist, move_dir.y * orbit_dist, 3.5);
        }
    }
}

pub fn camera_follow(
    player_q: Query<&Transform, With<Player>>,
    pm: Res<PlayerMovement>,
    mut cam_q: Query<(&mut Transform, &mut OrthographicProjection), (With<MainCamera>, Without<Player>)>,
    time: Res<Time>,
) {
    let Ok(ptf) = player_q.get_single() else { return };
    let Ok((mut ctf, mut proj)) = cam_q.get_single_mut() else { return };
    let dt = time.delta_secs();

    // Smooth camera follow
    let target = Vec3::new(ptf.translation.x, ptf.translation.y, ctf.translation.z);
    // Slightly lag camera more when dashing for a sense of speed
    let follow_speed = if pm.dashing { 6.0 } else { 8.0 };
    ctf.translation = ctf.translation.lerp(target, 1. - (-follow_speed * dt).exp());

    // Dynamic zoom: zoom out slightly at high speed, anchored to user's manual base_zoom
    let speed = if pm.dashing { DASH_SPEED } else { pm.velocity.length() };
    let target_scale = pm.base_zoom + (speed / (PLAYER_SPEED * SPRINT_MULT)).clamp(0., 1.) * 0.12;
    proj.scale = proj.scale + (target_scale - proj.scale) * (1. - (-5. * dt).exp());
}

pub fn camera_zoom(
    mut scroll_evr: EventReader<MouseWheel>,
    mut pm: ResMut<PlayerMovement>,
) {
    for ev in scroll_evr.read() {
        let delta = match ev.unit {
            MouseScrollUnit::Line  => ev.y * 0.10,
            MouseScrollUnit::Pixel => ev.y * 0.003,
        };
        pm.base_zoom = (pm.base_zoom - delta).clamp(0.35, 2.5);
    }
}
