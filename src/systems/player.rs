#![allow(clippy::type_complexity)]

use crate::components::{BodyPart, MainCamera, Player, PlayerIndicator};
use crate::constants::{
    PLAYER_ACCEL, PLAYER_FRICTION, PLAYER_SPEED, SPRINT_ENERGY_DRAIN, SPRINT_MULTIPLIER,
    WORLD_BOUNDARY,
};
use crate::resources::{
    ActionPrompt, BankInput, GameTime, PlayerMovement, PlayerStats, Transport, TransportKind,
    VehicleState,
};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

/// Pure computation: max speed given health, energy, transport, and sprint state.
pub fn compute_max_speed(
    health: f32,
    energy: f32,
    transport: &TransportKind,
    sprinting: bool,
) -> f32 {
    let health_mult = if health < 30. {
        0.4
    } else if energy < 20. {
        0.6
    } else {
        1.0
    };
    let transport_mult = match transport {
        TransportKind::Bike => 1.15,
        TransportKind::Car => 1.30,
        TransportKind::Walk => 1.0,
    };
    let sprint_mult = if sprinting { SPRINT_MULTIPLIER } else { 1.0 };
    PLAYER_SPEED * health_mult * transport_mult * sprint_mult
}

/// Pure computation: clamp a position axis within the world boundary.
pub fn clamp_position(pos: f32, velocity: f32, dt: f32) -> f32 {
    (pos + velocity * dt).clamp(-WORLD_BOUNDARY, WORLD_BOUNDARY)
}

/// Pure computation: drain energy while sprinting, clamped to 0.
pub fn sprint_drain(energy: f32, dt: f32) -> f32 {
    (energy - SPRINT_ENERGY_DRAIN * dt).max(0.)
}

pub fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    transport: Res<Transport>,
    mut q: Query<
        (
            &mut Transform,
            &mut PlayerMovement,
            &VehicleState,
            &BankInput,
            &ActionPrompt,
            &mut PlayerStats,
        ),
        With<Player>,
    >,
) {
    let Ok((mut tf, mut pm, vehicle_state, bank_input, action_prompt, mut stats)) = q.get_single_mut() else {
        return;
    };
    if vehicle_state.in_vehicle {
        return;
    }
    if bank_input.active || action_prompt.active {
        return;
    }
    let dt = time.delta_secs();

    // ── Input direction ────────────────────────────────────────────────────
    let mut wish = Vec2::ZERO;
    if keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        wish.y += 1.;
    }
    if keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        wish.y -= 1.;
    }
    if keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        wish.x -= 1.;
    }
    if keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        wish.x += 1.;
    }
    let sprinting = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
        && wish != Vec2::ZERO
        && stats.energy > 5.;
    pm.sprinting = sprinting;

    // ── Compute max speed ──────────────────────────────────────────────────
    let max_speed = compute_max_speed(stats.health, stats.energy, &transport.kind, sprinting);

    // ── Accelerate / friction ──────────────────────────────────────────────
    if wish != Vec2::ZERO {
        pm.velocity += wish.normalize() * PLAYER_ACCEL * dt;
        if pm.velocity.length() > max_speed {
            pm.velocity = pm.velocity.normalize() * max_speed;
        }
    } else {
        let speed = pm.velocity.length();
        let friction = (PLAYER_FRICTION * dt).min(speed);
        if speed > 0. {
            let dir = pm.velocity / speed;
            pm.velocity -= dir * friction;
        }
    }

    // ── Sprint energy drain ────────────────────────────────────────────────
    if sprinting {
        stats.energy = sprint_drain(stats.energy, dt);
    }

    // ── Apply movement (snapshot prev pos first for sub-step collision) ────
    pm.prev_position = tf.translation.truncate();
    if pm.velocity.length() > 0.5 {
        tf.translation.x = clamp_position(tf.translation.x, pm.velocity.x, dt);
        tf.translation.y = clamp_position(tf.translation.y, pm.velocity.y, dt);
    } else {
        pm.velocity = Vec2::ZERO;
    }
}

/// Squash/stretch via root scale; animate legs; tint body by state; orbit direction dot.
pub fn player_visuals(
    gt: Res<GameTime>,
    mut player_q: Query<(&Children, &mut Transform, &PlayerMovement, &PlayerStats), With<Player>>,
    mut parts_q: Query<
        (&BodyPart, &mut Sprite, &mut Transform),
        (Without<Player>, Without<PlayerIndicator>),
    >,
    mut indicator_q: Query<&mut Transform, (With<PlayerIndicator>, Without<Player>)>,
) {
    let Ok((children, mut root_tf, pm, stats)) = player_q.get_single_mut() else {
        return;
    };
    let speed = pm.velocity.length();
    let speed_norm = (speed / (PLAYER_SPEED * SPRINT_MULTIPLIER)).clamp(0., 1.);
    let t = gt.anim_secs;

    // Squash/stretch bob
    let bob = if speed > 10. {
        (t * (7. + speed * 0.012)).sin() * speed_norm * 0.06
    } else {
        0.
    };
    root_tf.scale = Vec3::new(
        1. - speed_norm * 0.08 - bob * 0.4,
        1. + speed_norm * 0.12 + bob,
        1.,
    );

    // Body tint: critical flash > low health > low energy > sprint glow > normal
    let body_color = if stats.critical_timer > 0. {
        let f = (t * 8.).sin() * 0.4 + 0.6;
        Color::srgb(1., f * 0.12, f * 0.04)
    } else if stats.health < 30. {
        Color::srgb(0.82, 0.28, 0.14)
    } else if stats.energy < 20. {
        Color::srgb(0.70, 0.65, 0.48)
    } else if pm.sprinting {
        let f = (t * 12.).sin() * 0.15 + 0.85;
        Color::srgb(1.0, 0.72 + f * 0.12, 0.30 + f * 0.10)
    } else {
        Color::srgb(0.90, 0.52, 0.12)
    };

    let leg_phase = (t * (7. + speed * 0.015)).sin() * speed_norm * 3.5;
    let move_dir = if speed > 10. {
        pm.velocity.normalize()
    } else {
        Vec2::Y
    };

    for &child in children.iter() {
        if let Ok((part, mut sprite, mut ctf)) = parts_q.get_mut(child) {
            match *part {
                BodyPart::Body => {
                    sprite.color = body_color;
                }
                BodyPart::LeftLeg => {
                    ctf.translation.y = -20. + leg_phase;
                }
                BodyPart::RightLeg => {
                    ctf.translation.y = -20. - leg_phase;
                }
                BodyPart::LeftFoot => {
                    ctf.translation.y = -40. + leg_phase * 0.65;
                }
                BodyPart::RightFoot => {
                    ctf.translation.y = -40. - leg_phase * 0.65;
                }
                _ => {}
            }
        }
        if let Ok(mut ctf) = indicator_q.get_mut(child) {
            ctf.translation = Vec3::new(move_dir.x * 72., move_dir.y * 72., 3.5);
        }
    }
}

pub fn camera_follow(
    player_q: Query<(&Transform, &PlayerMovement), With<Player>>,
    mut cam_q: Query<
        (&mut Transform, &mut OrthographicProjection),
        (With<MainCamera>, Without<Player>),
    >,
    time: Res<Time>,
) {
    let Ok((ptf, pm)) = player_q.get_single() else {
        return;
    };
    let Ok((mut ctf, mut proj)) = cam_q.get_single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    let target = Vec3::new(ptf.translation.x, ptf.translation.y, ctf.translation.z);
    ctf.translation = ctf.translation.lerp(target, 1. - (-8.0_f32 * dt).exp());

    let target_scale = pm.base_zoom
        + (pm.velocity.length() / (PLAYER_SPEED * SPRINT_MULTIPLIER)).clamp(0., 1.) * 0.10;
    proj.scale += (target_scale - proj.scale) * (1. - (-5.0_f32 * dt).exp());
}

pub fn camera_zoom(
    mut scroll_evr: EventReader<MouseWheel>,
    mut pm_q: Query<&mut PlayerMovement, With<Player>>,
) {
    let Ok(mut pm) = pm_q.get_single_mut() else {
        return;
    };
    for ev in scroll_evr.read() {
        let delta = match ev.unit {
            MouseScrollUnit::Line => ev.y * 0.10,
            MouseScrollUnit::Pixel => ev.y * 0.003,
        };
        pm.base_zoom = (pm.base_zoom - delta).clamp(1.0, 10.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_speed_walk_no_sprint() {
        let s = compute_max_speed(100., 100., &TransportKind::Walk, false);
        assert!((s - PLAYER_SPEED).abs() < f32::EPSILON);
    }

    #[test]
    fn max_speed_sprint_multiplier() {
        let s = compute_max_speed(100., 100., &TransportKind::Walk, true);
        assert!((s - PLAYER_SPEED * SPRINT_MULTIPLIER).abs() < f32::EPSILON);
    }

    #[test]
    fn max_speed_low_health_penalty() {
        let s = compute_max_speed(20., 100., &TransportKind::Walk, false);
        assert!((s - PLAYER_SPEED * 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn max_speed_low_energy_penalty() {
        let s = compute_max_speed(100., 10., &TransportKind::Walk, false);
        assert!((s - PLAYER_SPEED * 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn max_speed_bike_transport() {
        let s = compute_max_speed(100., 100., &TransportKind::Bike, false);
        assert!((s - PLAYER_SPEED * 1.15).abs() < f32::EPSILON);
    }

    #[test]
    fn max_speed_car_transport() {
        let s = compute_max_speed(100., 100., &TransportKind::Car, false);
        assert!((s - PLAYER_SPEED * 1.30).abs() < f32::EPSILON);
    }

    #[test]
    fn clamp_within_boundary() {
        let pos = clamp_position(0., 100., 0.016);
        assert!(pos > 0. && pos <= WORLD_BOUNDARY);
    }

    #[test]
    fn clamp_at_boundary() {
        let pos = clamp_position(WORLD_BOUNDARY - 1., 10000., 1.0);
        assert!((pos - WORLD_BOUNDARY).abs() < f32::EPSILON);
    }

    #[test]
    fn clamp_negative_boundary() {
        let pos = clamp_position(-WORLD_BOUNDARY + 1., -10000., 1.0);
        assert!((pos - (-WORLD_BOUNDARY)).abs() < f32::EPSILON);
    }

    #[test]
    fn sprint_drain_reduces_energy() {
        let e = sprint_drain(50., 0.016);
        assert!(e < 50.);
        assert!((e - (50. - SPRINT_ENERGY_DRAIN * 0.016)).abs() < f32::EPSILON);
    }

    #[test]
    fn sprint_drain_floors_at_zero() {
        let e = sprint_drain(0.001, 100.);
        assert!((e - 0.).abs() < f32::EPSILON);
    }
}
