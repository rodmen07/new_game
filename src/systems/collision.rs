use bevy::prelude::*;
use crate::components::{Player, Collider};
use crate::resources::PlayerMovement;

/// Player AABB half-extents — slightly forgiving (9px) vs visual 10px.
const PLAYER_HALF: Vec2 = Vec2::new(9., 9.);

/// Resolves AABB collisions between the player and all `Collider` entities.
/// Runs after `player_movement` so it corrects the position each frame.
/// Two resolution passes improve stability against corner cases and stacked colliders.
pub fn resolve_collisions(
    mut pm: ResMut<PlayerMovement>,
    mut player_q: Query<&mut Transform, With<Player>>,
    colliders_q: Query<(&Transform, &Collider), Without<Player>>,
) {
    let Ok(mut ptf) = player_q.get_single_mut() else { return };

    // Collect once, then run two resolution passes for stability.
    let colliders: Vec<_> = colliders_q.iter().collect();

    for _pass in 0..2 {
        for (ctf, collider) in &colliders {
            let ch = collider.0;
            let dx = ptf.translation.x - ctf.translation.x;
            let dy = ptf.translation.y - ctf.translation.y;
            let overlap_x = (PLAYER_HALF.x + ch.x) - dx.abs();
            let overlap_y = (PLAYER_HALF.y + ch.y) - dy.abs();

            if overlap_x <= 0. || overlap_y <= 0. {
                continue;
            }

            // Push out along axis of least penetration
            if overlap_x < overlap_y {
                let sign = if dx >= 0. { 1. } else { -1. };
                ptf.translation.x += overlap_x * sign;
                if pm.velocity.x * sign < 0. { pm.velocity.x = 0.; }
            } else {
                let sign = if dy >= 0. { 1. } else { -1. };
                ptf.translation.y += overlap_y * sign;
                if pm.velocity.y * sign < 0. { pm.velocity.y = 0.; }
            }

            // Cancel an active dash on collision
            if pm.dashing {
                pm.dashing = false;
                pm.velocity = Vec2::ZERO;
                pm.dash_cooldown = 0.40;
            }
        }
    }
}
