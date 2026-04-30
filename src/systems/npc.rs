use crate::components::{
    BodyPart, Collider, Interactable, LocalPlayer, Npc, NpcId, NpcLabel, NpcPersonality,
};
use crate::constants::{
    INTERACT_RADIUS, MAP_SCALE, NPC_SPEED, PLAYER_HOME_BOTTOM, PLAYER_HOME_LEFT, PLAYER_HOME_RIGHT,
    PLAYER_HOME_TOP,
};
use crate::resources::{GameTime, NearbyInteractable, NpcFriendship, QuestBoard};
use bevy::prelude::*;

const HOME_BUFFER: f32 = 12.;

fn keep_npc_out_of_player_house(npc: &mut Npc, tf: &mut Transform) {
    let x = tf.translation.x;
    let y = tf.translation.y;
    if !(PLAYER_HOME_LEFT..=PLAYER_HOME_RIGHT).contains(&x)
        || !(PLAYER_HOME_BOTTOM..=PLAYER_HOME_TOP).contains(&y)
    {
        return;
    }

    let left_gap = (x - PLAYER_HOME_LEFT).abs();
    let right_gap = (PLAYER_HOME_RIGHT - x).abs();
    let bottom_gap = (y - PLAYER_HOME_BOTTOM).abs();
    let top_gap = (PLAYER_HOME_TOP - y).abs();

    if left_gap <= right_gap && left_gap <= bottom_gap && left_gap <= top_gap {
        tf.translation.x = PLAYER_HOME_LEFT - HOME_BUFFER;
    } else if right_gap <= bottom_gap && right_gap <= top_gap {
        tf.translation.x = PLAYER_HOME_RIGHT + HOME_BUFFER;
    } else if bottom_gap <= top_gap {
        tf.translation.y = PLAYER_HOME_BOTTOM - HOME_BUFFER;
    } else {
        tf.translation.y = PLAYER_HOME_TOP + HOME_BUFFER;
    }

    npc.target = npc.zone_center;
    npc.velocity = Vec2::ZERO;
}

pub fn lcg(s: &mut u64) -> f32 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*s >> 33) as f32 / (u32::MAX as f32)
}

pub fn npc_wander(
    mut npc_q: Query<(&mut Npc, &mut Transform)>,
    time: Res<Time>,
    gt: Res<GameTime>,
) {
    let h = gt.hours;
    for (mut npc, mut tf) in &mut npc_q {
        // Select zone based on time of day
        let active_center = if (9. ..17.).contains(&h) {
            npc.work_zone
        } else if !(6. ..21.).contains(&h) {
            npc.home_zone
        } else {
            npc.zone_center // evening social zone
        };

        npc.wander_timer -= time.delta_secs();
        if npc.wander_timer <= 0. {
            let angle = lcg(&mut npc.rng) * std::f32::consts::TAU;
            let dist = lcg(&mut npc.rng) * npc.zone_half;
            let candidate = active_center + Vec2::new(angle.cos() * dist, angle.sin() * dist);
            // Never target inside the player's home zone
            npc.target = if (PLAYER_HOME_LEFT..=PLAYER_HOME_RIGHT).contains(&candidate.x)
                && (PLAYER_HOME_BOTTOM..=PLAYER_HOME_TOP).contains(&candidate.y)
            {
                active_center
            } else {
                candidate
            };
            npc.wander_timer = 2. + lcg(&mut npc.rng) * 4.;
        }
        let to = npc.target - tf.translation.truncate();
        if to.length() > 3. {
            let mv = to.normalize() * NPC_SPEED * time.delta_secs();
            tf.translation.x += mv.x;
            tf.translation.y += mv.y;
            npc.velocity = to.normalize() * NPC_SPEED;
        } else {
            npc.velocity = Vec2::ZERO;
        }

        keep_npc_out_of_player_house(&mut npc, &mut tf);
    }
}

/// NPC half-extents for AABB collision (matches player size).
const NPC_HALF: Vec2 = Vec2::new(9., 9.);

/// Computes the minimum-translation correction to push `entity_pos` out of a
/// collider rectangle. Returns `None` when there is no overlap.
///
/// Both positions are centres; half-extents describe half the width/height.
pub(crate) fn resolve_aabb_push(
    entity_pos: Vec2,
    entity_half: Vec2,
    collider_pos: Vec2,
    collider_half: Vec2,
) -> Option<Vec2> {
    let dx = entity_pos.x - collider_pos.x;
    let dy = entity_pos.y - collider_pos.y;
    let overlap_x = (entity_half.x + collider_half.x) - dx.abs();
    let overlap_y = (entity_half.y + collider_half.y) - dy.abs();

    if overlap_x <= 0. || overlap_y <= 0. {
        return None;
    }

    if overlap_x < overlap_y {
        let sign = if dx >= 0. { 1. } else { -1. };
        Some(Vec2::new(overlap_x * sign, 0.))
    } else {
        let sign = if dy >= 0. { 1. } else { -1. };
        Some(Vec2::new(0., overlap_y * sign))
    }
}

/// Resolves AABB collisions between all NPCs and `Collider` entities.
/// Single-pass resolution is sufficient since NPCs move slowly.
pub fn npc_collisions(
    mut npc_q: Query<&mut Transform, With<Npc>>,
    colliders_q: Query<(&Transform, &Collider), Without<Npc>>,
) {
    for mut ntf in &mut npc_q {
        for (ctf, collider) in &colliders_q {
            let npc_pos = ntf.translation.truncate();
            let collider_pos = ctf.translation.truncate();
            if let Some(delta) = resolve_aabb_push(npc_pos, NPC_HALF, collider_pos, collider.0) {
                ntf.translation.x += delta.x;
                ntf.translation.y += delta.y;
            }
        }
    }
}

/// Animates NPC body parts (leg walk cycle) based on tracked velocity.
pub fn npc_visuals(
    gt: Res<GameTime>,
    npc_q: Query<(&Npc, &Children)>,
    mut parts_q: Query<(&BodyPart, &mut Transform), Without<Npc>>,
) {
    let t = gt.anim_secs;
    for (npc, children) in &npc_q {
        let speed = npc.velocity.length();
        let speed_norm = (speed / NPC_SPEED).clamp(0., 1.);
        let leg_amp = speed_norm * 3. * MAP_SCALE;
        let leg_phase = (t * (6. + speed * 0.015)).sin() * leg_amp;

        for &child in children.iter() {
            if let Ok((part, mut ctf)) = parts_q.get_mut(child) {
                match *part {
                    BodyPart::LeftLeg => {
                        ctf.translation.y = -5. * MAP_SCALE + leg_phase;
                    }
                    BodyPart::RightLeg => {
                        ctf.translation.y = -5. * MAP_SCALE - leg_phase;
                    }
                    BodyPart::LeftFoot => {
                        ctf.translation.y = -10. * MAP_SCALE + leg_phase * 0.65;
                    }
                    BodyPart::RightFoot => {
                        ctf.translation.y = -10. * MAP_SCALE - leg_phase * 0.65;
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn update_npc_labels(
    npc_q: Query<&Transform, With<Npc>>,
    mut lbl: Query<(&NpcLabel, &mut Transform), Without<Npc>>,
) {
    for (label, mut ltf) in &mut lbl {
        if let Ok(ntf) = npc_q.get(label.0) {
            ltf.translation.x = ntf.translation.x;
            ltf.translation.y = ntf.translation.y + 26.;
        }
    }
}

pub fn update_npc_prompts(
    friendship: Res<NpcFriendship>,
    quest_board: Res<QuestBoard>,
    mut npc_inter_q: Query<(Entity, &Npc, &NpcId, &mut Interactable)>,
) {
    for (entity, npc, npc_id, mut inter) in &mut npc_inter_q {
        let lvl = *friendship.levels.get(&entity).unwrap_or(&0.) as u32;
        let tier = match lvl {
            0 => "Stranger",
            1 => "Acquaintance",
            2 => "Friend",
            3..=4 => "Close Friend",
            _ => "Best Friend",
        };
        let hearts = format!(
            "{}{}",
            "h".repeat(lvl.min(5) as usize),
            ".".repeat(5 - lvl.min(5) as usize)
        );
        let ptag = match npc.personality {
            NpcPersonality::Cheerful => " (Cheerful)",
            NpcPersonality::Wise => " (Wise)",
            NpcPersonality::Influential => " (Influential)",
            NpcPersonality::Neutral => "",
        };
        let quest_tag = if lvl >= 1
            && !quest_board.has_quest_from(npc_id.0)
            && quest_board.active_count() < 3
        {
            " [Q] Quest"
        } else {
            ""
        };
        let hangout_tag = if lvl >= 3 { " | [H] Hangout" } else { "" };
        inter.prompt = if lvl >= 2 {
            format!(
                "[E] Chat | [G] Gift -> {} [{}]{}{}{}",
                npc.name, tier, ptag, quest_tag, hangout_tag
            )
        } else {
            format!(
                "[E] Chat {} [{}]{} {}{}",
                npc.name, tier, ptag, hearts, quest_tag
            )
        };
    }
}

pub fn detect_nearby(
    player_q: Query<&Transform, With<LocalPlayer>>,
    inter_q: Query<(Entity, &Transform, &Interactable)>,
    mut nearby: ResMut<NearbyInteractable>,
) {
    let Some(ptf) = player_q.iter().next() else {
        return;
    };
    let pos = ptf.translation.truncate();
    nearby.entity = None;
    nearby.prompt.clear();
    let mut closest = f32::MAX;
    for (entity, tf, inter) in &inter_q {
        let d = pos.distance(tf.translation.truncate());
        if d < INTERACT_RADIUS && d < closest {
            closest = d;
            nearby.entity = Some(entity);
            nearby.prompt = inter.prompt.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_aabb_push;
    use bevy::math::Vec2;

    // ── resolve_aabb_push ─────────────────────────────────────────────────────

    #[test]
    fn no_overlap_returns_none() {
        // Entity at (0,0) half (5,5) vs collider at (20,0) half (5,5) - well clear
        let result = resolve_aabb_push(
            Vec2::ZERO,
            Vec2::splat(5.),
            Vec2::new(20., 0.),
            Vec2::splat(5.),
        );
        assert!(result.is_none());
    }

    #[test]
    fn overlap_from_right_pushes_entity_right() {
        // Entity centre at (8,0), collider centre at (0,0), both half=(5,5)
        // overlap_x = 10-8 = 2, overlap_y = 10-0 = 10 => push along x
        let delta = resolve_aabb_push(
            Vec2::new(8., 0.),
            Vec2::splat(5.),
            Vec2::ZERO,
            Vec2::splat(5.),
        )
        .unwrap();
        assert!(delta.x > 0., "should push right, got {delta:?}");
        assert!((delta.y).abs() < f32::EPSILON, "y component should be zero");
        assert!((delta.x - 2.).abs() < f32::EPSILON);
    }

    #[test]
    fn overlap_from_left_pushes_entity_left() {
        let delta = resolve_aabb_push(
            Vec2::new(-8., 0.),
            Vec2::splat(5.),
            Vec2::ZERO,
            Vec2::splat(5.),
        )
        .unwrap();
        assert!(delta.x < 0.);
        assert!((delta.x + 2.).abs() < f32::EPSILON);
    }

    #[test]
    fn overlap_from_above_pushes_entity_up() {
        // Vertical overlap is smaller -> push along y
        // Entity at (0,8), collider at (0,0), both half=(5,5)
        // overlap_x = 10-0 = 10, overlap_y = 10-8 = 2 => push along y
        let delta = resolve_aabb_push(
            Vec2::new(0., 8.),
            Vec2::splat(5.),
            Vec2::ZERO,
            Vec2::splat(5.),
        )
        .unwrap();
        assert!(delta.y > 0.);
        assert!((delta.y - 2.).abs() < f32::EPSILON);
        assert!((delta.x).abs() < f32::EPSILON);
    }

    #[test]
    fn exact_boundary_no_overlap() {
        // overlap_x = 10 - 10 = 0 -> no overlap
        let result = resolve_aabb_push(
            Vec2::new(10., 0.),
            Vec2::splat(5.),
            Vec2::ZERO,
            Vec2::splat(5.),
        );
        assert!(result.is_none());
    }

    #[test]
    fn asymmetric_half_extents() {
        // entity half=(4,4), collider half=(6,6), entity at (9,0)
        // overlap_x = (4+6) - 9 = 1, overlap_y = 10 - 0 = 10 => push along x
        let delta = resolve_aabb_push(
            Vec2::new(9., 0.),
            Vec2::new(4., 4.),
            Vec2::ZERO,
            Vec2::new(6., 6.),
        )
        .unwrap();
        assert!((delta.x - 1.).abs() < f32::EPSILON);
    }
}
