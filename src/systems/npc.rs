use bevy::prelude::*;
use crate::constants::{NPC_SPEED, INTERACT_RADIUS};
use crate::components::{Npc, NpcLabel, Interactable, Player, BodyPart};
use crate::resources::{NpcFriendship, NearbyInteractable};

pub fn lcg(s: &mut u64) -> f32 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*s >> 33) as f32 / (u32::MAX as f32)
}

pub fn npc_wander(mut npc_q: Query<(&mut Npc, &mut Transform)>, time: Res<Time>) {
    for (mut npc, mut tf) in &mut npc_q {
        npc.wander_timer -= time.delta_secs();
        if npc.wander_timer <= 0. {
            let angle = lcg(&mut npc.rng) * std::f32::consts::TAU;
            let dist  = lcg(&mut npc.rng) * npc.zone_half;
            npc.target = npc.zone_center + Vec2::new(angle.cos()*dist, angle.sin()*dist);
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
    }
}

/// Animates NPC body parts (leg walk cycle) based on tracked velocity.
pub fn npc_visuals(
    time: Res<Time>,
    npc_q: Query<(&Npc, &Children)>,
    mut parts_q: Query<(&BodyPart, &mut Transform), Without<Npc>>,
) {
    let t = time.elapsed_secs();
    for (npc, children) in &npc_q {
        let speed = npc.velocity.length();
        let speed_norm = (speed / NPC_SPEED).clamp(0., 1.);
        let leg_amp   = speed_norm * 3.;
        let leg_phase = (t * (6. + speed * 0.015)).sin() * leg_amp;

        for &child in children.iter() {
            if let Ok((part, mut ctf)) = parts_q.get_mut(child) {
                match *part {
                    BodyPart::LeftLeg   => { ctf.translation.y = -5. + leg_phase; }
                    BodyPart::RightLeg  => { ctf.translation.y = -5. - leg_phase; }
                    BodyPart::LeftFoot  => { ctf.translation.y = -10. + leg_phase * 0.65; }
                    BodyPart::RightFoot => { ctf.translation.y = -10. - leg_phase * 0.65; }
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
    mut npc_inter_q: Query<(Entity, &Npc, &mut Interactable)>,
) {
    for (entity, npc, mut inter) in &mut npc_inter_q {
        let lvl = *friendship.levels.get(&entity).unwrap_or(&0.) as u32;
        let hearts = format!("{}{}", "h".repeat(lvl.min(5) as usize), ".".repeat(5-lvl.min(5) as usize));
        inter.prompt = if lvl >= 5 { format!("[E] Chat | [G] Gift -> {}", npc.name) }
                       else        { format!("[E] Chat {} {}", npc.name, hearts) };
    }
}

pub fn detect_nearby(
    player_q: Query<&Transform, With<Player>>,
    inter_q: Query<(Entity, &Transform, &Interactable)>,
    mut nearby: ResMut<NearbyInteractable>,
) {
    let Ok(ptf) = player_q.get_single() else { return };
    let pos = ptf.translation.truncate();
    nearby.entity = None; nearby.prompt.clear();
    let mut closest = f32::MAX;
    for (entity, tf, inter) in &inter_q {
        let d = pos.distance(tf.translation.truncate());
        if d < INTERACT_RADIUS && d < closest { closest = d; nearby.entity = Some(entity); nearby.prompt = inter.prompt.clone(); }
    }
}
