#![allow(clippy::too_many_arguments)]

use crate::components::*;
use crate::resources::*;
use bevy::prelude::*;

/// Generate a quest for a specific NPC based on friendship level and personality.
fn generate_quest(npc_id: usize, friendship: f32, personality: NpcPersonality, day: u32) -> NpcQuest {
    let seed = (day.wrapping_mul(2654435761)).wrapping_add(npc_id as u32 * 999983);
    let variant = seed % 6;

    let (kind, target, reward_money, reward_friend) = match variant {
        0 => (QuestKind::FetchItem(ItemKind::Coffee, 2), 2, 15., 0.5),
        1 => (QuestKind::DoActivity(ActionKind::Exercise, 2), 2, 20., 0.4),
        2 => (QuestKind::EarnMoney(60.), 1, 10., 0.6),
        3 => (QuestKind::CraftItem(2), 2, 25., 0.5),
        4 => (QuestKind::FetchItem(ItemKind::Smoothie, 1), 1, 20., 0.6),
        _ => (QuestKind::DoActivity(ActionKind::Meditate, 1), 1, 15., 0.4),
    };

    // Scale rewards with friendship and personality
    let money_mult = match personality {
        NpcPersonality::Influential => 1.5,
        NpcPersonality::Wise => 1.2,
        _ => 1.0,
    };
    let friend_mult = match personality {
        NpcPersonality::Cheerful => 1.5,
        NpcPersonality::Wise => 1.3,
        _ => 1.0,
    };
    let tier_bonus = (friendship * 0.1).min(0.5);

    let npc_name = match npc_id {
        0 => "Alex",
        1 => "Sam",
        2 => "Mia",
        3 => "Jordan",
        4 => "Taylor",
        5 => "Casey",
        _ => "NPC",
    };

    NpcQuest {
        npc_id,
        description: format!("{}: {}", npc_name, kind.description()),
        kind,
        reward_money: (reward_money * money_mult * (1. + tier_bonus)).round(),
        reward_friendship: reward_friend * friend_mult,
        progress: 0,
        target,
        completed: false,
    }
}

/// Offer quests from NPCs when chatting - called from quest_offer_system.
/// NPCs offer quests if friendship >= 1 and they don't already have an active quest.
pub fn quest_offer_system(
    quest_board: Res<QuestBoard>,
    friendship: Res<NpcFriendship>,
    gt: Res<GameTime>,
    npc_q: Query<(Entity, &NpcId, &Npc)>,
    nearby: Res<NearbyInteractable>,
    inter_q: Query<&Interactable>,
    mut notif: ResMut<Notification>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::KeyQ) {
        return;
    }

    let Some(entity) = nearby.entity else { return };
    let Ok(inter) = inter_q.get(entity) else { return };
    if !matches!(&inter.action, ActionKind::Chat) { return; }

    let Ok((_, npc_id, npc)) = npc_q.get(entity) else { return };
    let lvl = friendship.levels.get(&entity).copied().unwrap_or(0.);

    if lvl < 1. {
        notif.push("Build more friendship before they'll ask for help.", 2.5);
        return;
    }

    if quest_board.has_quest_from(npc_id.0) {
        notif.push(format!("{} already gave you a quest. Complete it first!", npc.name), 2.5);
        return;
    }

    if quest_board.active_count() >= 3 {
        notif.push("Quest log full (3 max). Complete a quest first!", 2.5);
        return;
    }

    let quest = generate_quest(npc_id.0, lvl, npc.personality, gt.day);
    notif.push(format!("New quest from {}! {}  Reward: ${:.0}", npc.name, quest.description, quest.reward_money), 5.);

    // We need mutable access to QuestBoard, so use commands
    commands.queue(move |world: &mut World| {
        world.resource_mut::<QuestBoard>().quests.push(quest);
    });
}

/// Track progress on active quests each frame.
pub fn quest_progress_system(
    mut quest_board: ResMut<QuestBoard>,
    mut gs: ResMut<GameState>,
    inv: Res<Inventory>,
    mut stats: ResMut<PlayerStats>,
    mut friendship: ResMut<NpcFriendship>,
    mut notif: ResMut<Notification>,
    npc_q: Query<(Entity, &NpcId)>,
) {
    let mut any_completed = false;
    let crafted_today = quest_board.crafted_today;

    for quest in quest_board.quests.iter_mut() {
        if quest.completed {
            continue;
        }

        // Update progress based on quest kind
        let current = match &quest.kind {
            QuestKind::FetchItem(item, _) => {
                match item {
                    ItemKind::Coffee => inv.coffee,
                    ItemKind::Vitamins => inv.vitamins,
                    ItemKind::Books => inv.books,
                    ItemKind::Ingredient => inv.ingredient,
                    ItemKind::GiftBox => inv.gift_box,
                    ItemKind::Smoothie => inv.smoothie,
                }
            }
            QuestKind::DoActivity(action, _) => {
                match action {
                    ActionKind::Work => gs.work_today,
                    ActionKind::Exercise => gs.exercise_today,
                    ActionKind::Chat => gs.chat_today,
                    ActionKind::Eat => gs.eat_today,
                    ActionKind::Hobby(_) => gs.hobby_today,
                    ActionKind::GymSession => gs.exercise_today,
                    ActionKind::Meditate => gs.study_today, // reuse study counter
                    _ => 0,
                }
            }
            QuestKind::EarnMoney(_) => {
                if gs.money_earned_today >= 60. { 1 } else { 0 }
            }
            QuestKind::CraftItem(_) => crafted_today,
        };

        quest.progress = current.min(quest.target);

        if quest.progress >= quest.target && !quest.completed {
            quest.completed = true;
            any_completed = true;

            stats.money += quest.reward_money;
            gs.total_quests += 1;

            // Apply friendship reward
            let npc_entity = npc_q.iter().find(|(_, id)| id.0 == quest.npc_id).map(|(e, _)| e);
            if let Some(e) = npc_entity {
                let f = friendship.levels.entry(e).or_insert(0.);
                *f = (*f + quest.reward_friendship).min(5.);
            }

            let npc_name = match quest.npc_id {
                0 => "Alex", 1 => "Sam", 2 => "Mia",
                3 => "Jordan", 4 => "Taylor", _ => "Casey",
            };
            notif.push(
                format!("Quest complete for {}! +${:.0} +{:.1} friendship", npc_name, quest.reward_money, quest.reward_friendship),
                5.,
            );
        }
    }

    if any_completed {
        quest_board.completed_total += quest_board.quests.iter().filter(|q| q.completed).count() as u32;
        quest_board.quests.retain(|q| !q.completed);
    }
}
