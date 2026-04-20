#![allow(clippy::type_complexity)]

use crate::components::{Player, Vehicle};
use crate::resources::{ActionPrompt, PlayerMovement, Transport, TransportKind, VehicleState};
use bevy::prelude::*;

const CAR_SPEED: f32 = 340.;
const CAR_ACCEL: f32 = 800.;
const CAR_FRICTION: f32 = 400.;

pub fn car_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_q: Query<
        (
            &mut Transform,
            &mut PlayerMovement,
            &VehicleState,
            &ActionPrompt,
        ),
        (With<Player>, Without<Vehicle>),
    >,
    mut car_q: Query<&mut Transform, (With<Vehicle>, Without<Player>)>,
) {
    let Ok((mut ptf, mut pm, vehicle_state, action_prompt)) = player_q.get_single_mut() else {
        return;
    };
    if action_prompt.active || !vehicle_state.in_vehicle {
        return;
    }
    let dt = time.delta_secs();

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

    if wish != Vec2::ZERO {
        pm.velocity += wish.normalize() * CAR_ACCEL * dt;
        if pm.velocity.length() > CAR_SPEED {
            pm.velocity = pm.velocity.normalize() * CAR_SPEED;
        }
    } else {
        let speed = pm.velocity.length();
        let friction = (CAR_FRICTION * dt).min(speed);
        if speed > 0. {
            let dir = pm.velocity / speed;
            pm.velocity -= dir * friction;
        }
    }

    if pm.velocity.length() > 0.5 {
        ptf.translation.x = (ptf.translation.x + pm.velocity.x * dt).clamp(-1600., 1600.);
        ptf.translation.y = (ptf.translation.y + pm.velocity.y * dt).clamp(-1600., 1600.);
    } else {
        pm.velocity = Vec2::ZERO;
    }

    // Sync car entity position to player
    if let Ok(mut ctf) = car_q.get_single_mut() {
        ctf.translation.x = ptf.translation.x;
        ctf.translation.y = ptf.translation.y;
        ctf.translation.z = ptf.translation.z - 1.;
    }
}

pub fn reveal_car_on_purchase(
    transport: Res<Transport>,
    mut car_q: Query<&mut Visibility, With<Vehicle>>,
) {
    if !transport.is_changed() {
        return;
    }
    if transport.kind == TransportKind::Car {
        for mut vis in &mut car_q {
            *vis = Visibility::Visible;
        }
    }
}
