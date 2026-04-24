#![allow(clippy::too_many_arguments)]

use crate::components::*;
use crate::resources::*;
use bevy::prelude::*;

/// Generate a quest for a specific NPC based on friendship level and personality.
fn generate_quest(
    npc_id: usize,
    friendship: f32,
    personality: NpcPersonality,
    day: u32,
) -> NpcQuest {
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
    mut actions: EventReader<PlayerAction>,
    mut commands: Commands,
) {
    let mut requested = false;
    for action in actions.read() {
        if matches!(action, PlayerAction::RequestQuest) {
            requested = true;
        }
    }
    if !requested {
        return;
    }

    let Some(entity) = nearby.entity else { return };
    let Ok(inter) = inter_q.get(entity) else {
        return;
    };
    if !matches!(&inter.action, ActionKind::Chat) {
        return;
    }

    let Ok((_, npc_id, npc)) = npc_q.get(entity) else {
        return;
    };
    let lvl = friendship.levels.get(&entity).copied().unwrap_or(0.);

    if lvl < 1. {
        notif.push("Build more friendship before they'll ask for help.", 2.5);
        return;
    }

    if quest_board.has_quest_from(npc_id.0) {
        notif.push(
            format!("{} already gave you a quest. Complete it first!", npc.name),
            2.5,
        );
        return;
    }

    if quest_board.active_count() >= 3 {
        notif.push("Quest log full (3 max). Complete a quest first!", 2.5);
        return;
    }

    let quest = generate_quest(npc_id.0, lvl, npc.personality, gt.day);
    notif.push(
        format!(
            "New quest from {}! {}  Reward: ${:.0}",
            npc.name, quest.description, quest.reward_money
        ),
        5.,
    );

    // We need mutable access to QuestBoard, so use commands
    commands.queue(move |world: &mut World| {
        world.resource_mut::<QuestBoard>().quests.push(quest);
    });
}

/// Track progress on active quests each frame.
pub fn quest_progress_system(
    mut quest_board: ResMut<QuestBoard>,
    mut gs: ResMut<GameState>,
    mut player_q: Query<(&Inventory, &mut PlayerStats), With<LocalPlayer>>,
    mut friendship: ResMut<NpcFriendship>,
    mut notif: ResMut<Notification>,
    npc_q: Query<(Entity, &NpcId)>,
) {
    let Some((inv, mut stats)) = player_q.iter_mut().next() else {
        return;
    };
    let mut any_completed = false;
    let crafted_today = quest_board.crafted_today;

    for quest in quest_board.quests.iter_mut() {
        if quest.completed {
            continue;
        }

        // Update progress based on quest kind
        let current = match &quest.kind {
            QuestKind::FetchItem(item, _) => match item {
                ItemKind::Coffee => inv.coffee,
                ItemKind::Vitamins => inv.vitamins,
                ItemKind::Books => inv.books,
                ItemKind::Ingredient => inv.ingredient,
                ItemKind::GiftBox => inv.gift_box,
                ItemKind::Smoothie => inv.smoothie,
            },
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
            QuestKind::EarnMoney(amount) => {
                if gs.money_earned_today >= *amount {
                    1
                } else {
                    0
                }
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
            let npc_entity = npc_q
                .iter()
                .find(|(_, id)| id.0 == quest.npc_id)
                .map(|(e, _)| e);
            if let Some(e) = npc_entity {
                let f = friendship.levels.entry(e).or_insert(0.);
                *f = (*f + quest.reward_friendship).clamp(0., 5.);
            }

            let npc_name = match quest.npc_id {
                0 => "Alex",
                1 => "Sam",
                2 => "Mia",
                3 => "Jordan",
                4 => "Taylor",
                _ => "Casey",
            };
            notif.push(
                format!(
                    "Quest complete for {}! +${:.0} +{:.1} friendship",
                    npc_name, quest.reward_money, quest.reward_friendship
                ),
                5.,
            );
        }
    }

    if any_completed {
        quest_board.completed_total +=
            quest_board.quests.iter().filter(|q| q.completed).count() as u32;
        quest_board.quests.retain(|q| !q.completed);
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    fn quest(npc_id: usize, kind: QuestKind, target: u32) -> NpcQuest {
        NpcQuest {
            npc_id,
            description: format!("test #{}", npc_id),
            kind,
            reward_money: 25.,
            reward_friendship: 0.5,
            progress: 0,
            target,
            completed: false,
        }
    }

    fn build_app(quests: Vec<NpcQuest>) -> App {
        let mut app = App::new();
        let mut board = QuestBoard::default();
        board.quests = quests;
        app.insert_resource(board);
        app.insert_resource(GameState::default());
        app.insert_resource(NpcFriendship::default());
        app.insert_resource(Notification::default());
        app.add_systems(Update, quest_progress_system);
        app
    }

    fn spawn_player(app: &mut App, inv: Inventory) -> Entity {
        app.world_mut()
            .spawn((LocalPlayer, inv, PlayerStats::default()))
            .id()
    }

    #[test]
    fn fetch_item_progress_tracks_inventory_count() {
        let mut app = build_app(vec![quest(0, QuestKind::FetchItem(ItemKind::Coffee, 3), 3)]);
        let mut inv = Inventory::default();
        inv.coffee = 1;
        spawn_player(&mut app, inv);
        app.update();
        let board = app.world().resource::<QuestBoard>();
        // One incomplete quest with progress=1.
        assert_eq!(board.quests.len(), 1);
        assert_eq!(board.quests[0].progress, 1);
        assert!(!board.quests[0].completed);
    }

    #[test]
    fn fetch_item_completes_when_inventory_meets_target() {
        let mut app = build_app(vec![quest(
            0,
            QuestKind::FetchItem(ItemKind::Vitamins, 2),
            2,
        )]);
        let mut inv = Inventory::default();
        inv.vitamins = 5; // exceeds target
        spawn_player(&mut app, inv);
        app.update();
        let board = app.world().resource::<QuestBoard>();
        // Completed quests are removed from the active list.
        assert!(board.quests.is_empty(), "completed quest should be removed");
        assert_eq!(board.completed_total, 1);
    }

    #[test]
    fn completion_grants_money_reward_and_increments_total() {
        let q = quest(1, QuestKind::CraftItem(2), 2);
        let reward = q.reward_money;
        let mut app = build_app(vec![q]);
        let _ = spawn_player(&mut app, Inventory::default());
        app.world_mut().resource_mut::<QuestBoard>().crafted_today = 5;
        app.update();
        // Player should have received the reward.
        let mut q = app
            .world_mut()
            .query_filtered::<&PlayerStats, With<LocalPlayer>>();
        let stats = q.single(app.world());
        let starting = PlayerStats::default().money;
        assert!(
            (stats.money - (starting + reward)).abs() < 0.001,
            "expected {} got {}",
            starting + reward,
            stats.money
        );
        let gs = app.world().resource::<GameState>();
        assert_eq!(gs.total_quests, 1);
    }

    #[test]
    fn earn_money_quest_only_completes_when_threshold_met() {
        // Use a non-60 amount so the `EarnMoney(f32)` payload is actually
        // exercised (guards against regressions where the value is ignored).
        let mut app = build_app(vec![quest(2, QuestKind::EarnMoney(125.), 1)]);
        spawn_player(&mut app, Inventory::default());
        // Above the old hard-coded 60 but below the quest's real threshold → no progress.
        app.world_mut()
            .resource_mut::<GameState>()
            .money_earned_today = 100.;
        app.update();
        {
            let board = app.world().resource::<QuestBoard>();
            assert_eq!(board.quests[0].progress, 0);
            assert!(!board.quests[0].completed);
        }
        // At threshold → completes.
        app.world_mut()
            .resource_mut::<GameState>()
            .money_earned_today = 125.;
        app.update();
        let board = app.world().resource::<QuestBoard>();
        assert!(board.quests.is_empty());
        assert_eq!(board.completed_total, 1);
    }

    #[test]
    fn earn_money_quest_honors_amount_in_payload() {
        // A very small target should complete with a very small income, proving
        // the payload is read (not the old hard-coded 60).
        let mut app = build_app(vec![quest(0, QuestKind::EarnMoney(5.), 1)]);
        spawn_player(&mut app, Inventory::default());
        app.world_mut()
            .resource_mut::<GameState>()
            .money_earned_today = 10.;
        app.update();
        let board = app.world().resource::<QuestBoard>();
        assert!(board.quests.is_empty());
        assert_eq!(board.completed_total, 1);
    }

    #[test]
    fn do_activity_progress_uses_correct_counter() {
        let mut app = build_app(vec![
            quest(0, QuestKind::DoActivity(ActionKind::Work, 2), 2),
            quest(1, QuestKind::DoActivity(ActionKind::Exercise, 1), 1),
            quest(2, QuestKind::DoActivity(ActionKind::Chat, 3), 3),
            quest(3, QuestKind::DoActivity(ActionKind::Eat, 1), 1),
            quest(
                4,
                QuestKind::DoActivity(ActionKind::Hobby(HobbyKind::Painting), 1),
                1,
            ),
            quest(5, QuestKind::DoActivity(ActionKind::Meditate, 2), 2),
        ]);
        spawn_player(&mut app, Inventory::default());
        {
            let mut gs = app.world_mut().resource_mut::<GameState>();
            gs.work_today = 2;
            gs.exercise_today = 1;
            gs.chat_today = 3;
            gs.eat_today = 1;
            gs.hobby_today = 1;
            gs.study_today = 2; // Meditate reuses the study counter
        }
        app.update();
        let board = app.world().resource::<QuestBoard>();
        // All six should have completed and been removed.
        assert!(
            board.quests.is_empty(),
            "remaining: {:?}",
            board.quests.iter().map(|q| q.npc_id).collect::<Vec<_>>()
        );
        assert_eq!(board.completed_total, 6);
    }

    #[test]
    fn gym_session_action_maps_to_exercise_counter() {
        let mut app = build_app(vec![quest(
            0,
            QuestKind::DoActivity(ActionKind::GymSession, 1),
            1,
        )]);
        spawn_player(&mut app, Inventory::default());
        app.world_mut().resource_mut::<GameState>().exercise_today = 1;
        app.update();
        let board = app.world().resource::<QuestBoard>();
        assert!(board.quests.is_empty());
    }

    #[test]
    fn unrelated_action_kinds_do_not_advance_progress() {
        // ActionKind::Sleep falls into the catch-all `_ => 0` branch.
        let mut app = build_app(vec![quest(
            0,
            QuestKind::DoActivity(ActionKind::Sleep, 1),
            1,
        )]);
        spawn_player(&mut app, Inventory::default());
        app.world_mut().resource_mut::<GameState>().work_today = 99;
        app.update();
        let board = app.world().resource::<QuestBoard>();
        assert_eq!(board.quests.len(), 1);
        assert_eq!(board.quests[0].progress, 0);
    }

    #[test]
    fn no_player_entity_is_a_noop() {
        let mut app = build_app(vec![quest(0, QuestKind::EarnMoney(60.), 1)]);
        // No LocalPlayer spawned.
        app.world_mut()
            .resource_mut::<GameState>()
            .money_earned_today = 100.;
        app.update();
        let board = app.world().resource::<QuestBoard>();
        assert_eq!(board.quests.len(), 1, "should not progress without player");
        assert_eq!(board.quests[0].progress, 0);
    }

    #[test]
    fn already_completed_quests_are_skipped() {
        // A quest pre-marked completed should not be touched. The board only
        // prunes completed quests when at least one new completion happened.
        let mut q = quest(0, QuestKind::CraftItem(1), 1);
        q.completed = true;
        q.progress = 1;
        let mut app = build_app(vec![q]);
        spawn_player(&mut app, Inventory::default());
        app.update();
        let board = app.world().resource::<QuestBoard>();
        // No NEW completion happened, so completed_total stays 0 and the
        // already-completed quest is left alone in the list.
        assert_eq!(board.completed_total, 0);
        assert_eq!(board.quests.len(), 1);
        assert!(board.quests[0].completed);
    }

    #[test]
    fn friendship_reward_is_applied_to_matching_npc_entity() {
        let mut app = build_app(vec![quest(0, QuestKind::EarnMoney(60.), 1)]);
        spawn_player(&mut app, Inventory::default());
        app.world_mut()
            .resource_mut::<GameState>()
            .money_earned_today = 100.;
        // Spawn an NPC with the matching NpcId, register baseline friendship.
        let npc_entity = app.world_mut().spawn(NpcId(0)).id();
        app.world_mut()
            .resource_mut::<NpcFriendship>()
            .levels
            .insert(npc_entity, 1.0);
        app.update();
        let f = app.world().resource::<NpcFriendship>();
        let level = *f.levels.get(&npc_entity).unwrap();
        assert!((level - 1.5).abs() < 0.001, "got {}", level);
    }

    #[test]
    fn friendship_clamped_to_5() {
        let mut app = build_app(vec![quest(0, QuestKind::EarnMoney(60.), 1)]);
        spawn_player(&mut app, Inventory::default());
        app.world_mut()
            .resource_mut::<GameState>()
            .money_earned_today = 100.;
        let npc_entity = app.world_mut().spawn(NpcId(0)).id();
        app.world_mut()
            .resource_mut::<NpcFriendship>()
            .levels
            .insert(npc_entity, 4.9);
        app.update();
        let f = app.world().resource::<NpcFriendship>();
        let level = *f.levels.get(&npc_entity).unwrap();
        assert!((level - 5.0).abs() < f32::EPSILON, "got {}", level);
    }
}
