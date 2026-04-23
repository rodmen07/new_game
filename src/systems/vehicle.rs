#![allow(clippy::type_complexity)]

use bevy::prelude::*;

use crate::components::{LocalPlayer, OwnedPetVisual, PetKind, Player, Vehicle};
use crate::resources::{ActionPrompt, Pet, PlayerMovement, Transport, TransportKind, VehicleState};

const CAR_SPEED: f32 = 340.0;
const CAR_ACCEL: f32 = 800.0;
const CAR_FRICTION: f32 = 400.0;

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
        (With<LocalPlayer>, Without<Vehicle>),
    >,
    mut car_q: Query<&mut Transform, (With<Vehicle>, Without<Player>)>,
) {
    let Some((mut ptf, mut pm, vehicle_state, action_prompt)) = player_q.iter_mut().next() else {
        return;
    };

    if action_prompt.active || !vehicle_state.in_vehicle {
        return;
    }

    let dt = time.delta_secs();
    let mut wish = Vec2::ZERO;
    if keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        wish.y += 1.0;
    }
    if keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        wish.y -= 1.0;
    }
    if keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        wish.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        wish.x += 1.0;
    }

    if wish != Vec2::ZERO {
        pm.velocity += wish.normalize() * CAR_ACCEL * dt;
        if pm.velocity.length() > CAR_SPEED {
            pm.velocity = pm.velocity.normalize() * CAR_SPEED;
        }
    } else {
        let speed = pm.velocity.length();
        let friction = (CAR_FRICTION * dt).min(speed);
        if speed > 0.0 {
            let dir = pm.velocity / speed;
            pm.velocity -= dir * friction;
        }
    }

    if pm.velocity.length() > 0.5 {
        ptf.translation.x = (ptf.translation.x + pm.velocity.x * dt).clamp(-1600.0, 1600.0);
        ptf.translation.y = (ptf.translation.y + pm.velocity.y * dt).clamp(-1600.0, 1600.0);
    } else {
        pm.velocity = Vec2::ZERO;
    }

    if let Some(mut ctf) = car_q.iter_mut().next() {
        ctf.translation.x = ptf.translation.x;
        ctf.translation.y = ptf.translation.y;
        ctf.translation.z = ptf.translation.z - 1.0;
    }
}

pub fn reveal_car_on_purchase(
    transport: Res<Transport>,
    pet: Res<Pet>,
    mut car_q: Query<&mut Visibility, (With<Vehicle>, Without<OwnedPetVisual>)>,
    mut pet_q: Query<
        (&mut Visibility, &mut Sprite),
        (With<OwnedPetVisual>, Without<Vehicle>),
    >,
) {
    if transport.is_changed() {
        let show_car = transport.kind == TransportKind::Car;
        for mut vis in &mut car_q {
            *vis = if show_car {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }

    if pet.is_changed() {
        for (mut vis, mut sprite) in &mut pet_q {
            if pet.has_pet {
                *vis = Visibility::Visible;
                match pet.kind {
                    PetKind::Dog => {
                        sprite.color = Color::srgb(0.70, 0.54, 0.34);
                        sprite.custom_size = Some(Vec2::new(72., 44.));
                    }
                    PetKind::Cat => {
                        sprite.color = Color::srgb(0.82, 0.58, 0.24);
                        sprite.custom_size = Some(Vec2::new(64., 40.));
                    }
                    PetKind::Fish => {
                        sprite.color = Color::srgb(0.33, 0.68, 0.92);
                        sprite.custom_size = Some(Vec2::new(52., 28.));
                    }
                }
            } else {
                *vis = Visibility::Hidden;
            }
        }
    }
}
