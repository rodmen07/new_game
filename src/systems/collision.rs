use crate::components::{Collider, Player};
use crate::resources::PlayerMovement;
use bevy::prelude::*;

/// Player AABB half-extents — slightly forgiving (9px) vs visual 10px.
pub const PLAYER_HALF: Vec2 = Vec2::new(9., 9.);

/// Resolves AABB collisions between the player and all `Collider` entities.
/// Runs after `player_movement`. Sub-steps the frame displacement into increments
/// no larger than PLAYER_HALF.x to prevent tunneling through thin walls at sprint speed.
pub fn resolve_collisions(
    mut player_q: Query<(&mut Transform, &mut PlayerMovement), With<Player>>,
    colliders_q: Query<(&Transform, &Collider), Without<Player>>,
) {
    let Ok((mut ptf, mut pm)) = player_q.get_single_mut() else {
        return;
    };

    let colliders: Vec<_> = colliders_q.iter().collect();

    let curr = ptf.translation.truncate();
    let delta = curr - pm.prev_position;
    let dist = delta.length();

    // How many sub-steps — ceil(dist / half-extent), clamped to [1, 8].
    let step_count = ((dist / PLAYER_HALF.x).ceil() as usize).clamp(1, 8);
    let step = delta / step_count as f32;

    // Revert to pre-movement position then advance in increments.
    let start = pm.prev_position;
    ptf.translation.x = start.x;
    ptf.translation.y = start.y;

    for _ in 0..step_count {
        ptf.translation.x = (ptf.translation.x + step.x).clamp(-1600., 1600.);
        ptf.translation.y = (ptf.translation.y + step.y).clamp(-1600., 1600.);

        // Two resolution passes per step for corner/stacked stability.
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

                if overlap_x < overlap_y {
                    let sign = if dx >= 0. { 1. } else { -1. };
                    ptf.translation.x += overlap_x * sign;
                    if pm.velocity.x * sign < 0. {
                        pm.velocity.x = 0.;
                    }
                } else {
                    let sign = if dy >= 0. { 1. } else { -1. };
                    ptf.translation.y += overlap_y * sign;
                    if pm.velocity.y * sign < 0. {
                        pm.velocity.y = 0.;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PLAYER_HALF;

    fn step_count(dist: f32) -> usize {
        ((dist / PLAYER_HALF.x).ceil() as usize).clamp(1, 8)
    }

    #[test]
    fn zero_displacement_gives_one_step() {
        assert_eq!(step_count(0.), 1);
    }

    #[test]
    fn tiny_displacement_gives_one_step() {
        assert_eq!(step_count(1.), 1);
    }

    #[test]
    fn displacement_equal_to_half_extent_gives_one_step() {
        assert_eq!(step_count(PLAYER_HALF.x), 1);
    }

    #[test]
    fn displacement_just_over_half_extent_gives_two_steps() {
        assert_eq!(step_count(PLAYER_HALF.x + 0.1), 2);
    }

    #[test]
    fn sprint_at_60fps_stays_one_step() {
        // Max sprint: 200 * 1.75 = 350 px/s. At 60fps dt=0.0167s: 350*0.0167 ≈ 5.8px < PLAYER_HALF.x (9px)
        let dist = 350_f32 * (1. / 60.);
        assert_eq!(step_count(dist), 1);
    }

    #[test]
    fn large_displacement_clamped_to_eight_steps() {
        assert_eq!(step_count(9999.), 8);
    }

    #[test]
    fn step_count_is_at_least_one() {
        for dist in [0., 0.001, 0.5, 8.9, 9.0, 9.1, 18., 100., 9999.] {
            assert!(step_count(dist) >= 1, "step_count({dist}) was 0");
        }
    }

    #[test]
    fn step_count_never_exceeds_eight() {
        for dist in [0., 1., 100., 1000., f32::MAX / 2.] {
            assert!(step_count(dist) <= 8, "step_count({dist}) exceeded 8");
        }
    }

    #[test]
    fn aabb_overlap_pushes_on_least_penetration_axis() {
        // Verify the overlap formula: overlap_x = (PLAYER_HALF.x + ch.x) - |dx|
        let ch = bevy::math::Vec2::new(10., 10.);
        let dx = 5_f32; // player is 5px right of collider center
        let dy = 15_f32; // player is 15px above collider center
        let overlap_x = (PLAYER_HALF.x + ch.x) - dx.abs(); // 9+10-5 = 14
        let overlap_y = (PLAYER_HALF.y + ch.y) - dy.abs(); // 9+10-15 = 4
        // least penetration is Y axis (overlap_y < overlap_x), so push on Y
        assert!(overlap_x > 0. && overlap_y > 0., "should be overlapping");
        assert!(
            overlap_y < overlap_x,
            "should push on Y axis (least penetration)"
        );
    }
}
