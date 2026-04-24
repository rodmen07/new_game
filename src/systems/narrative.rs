use bevy::prelude::*;

use crate::components::LocalPlayer;
use crate::resources::*;

#[allow(clippy::too_many_arguments)]
pub fn update_narrative(
    gt: Res<GameTime>,
    _gs: Res<GameState>,
    player_q: Query<(&PlayerStats, &Skills, &HousingTier), With<LocalPlayer>>,
    friendship: Res<NpcFriendship>,
    rating: Res<LifeRating>,
    conds: Res<Conditions>,
    rep: Res<Reputation>,
    transport: Res<Transport>,
    mut story: ResMut<NarrativeState>,
    mut notif: ResMut<Notification>,
) {
    let Some((stats, skills, housing)) = player_q.iter().next() else {
        return;
    };
    let unlocked = (gt.day == 0)
        && story.unlock(
            "intro",
            "New in Town",
            "No home yet. Work to earn cash, then deposit $90 at the Bank to sign a lease. Short on energy? Rest at the park shelter for the night.",
        )
        || ((gt.day >= 1 || stats.money >= 140.)
            && story.unlock(
                "routine",
                "Finding a Rhythm",
                "The days are starting to rhyme. Work, meals, and rest are turning into a life.",
            ))
        || (friendship.levels.values().any(|&v| v >= 3.)
            && story.unlock(
                "friendship",
                "A Familiar Face",
                "One steady friendship makes the whole neighborhood feel less cold.",
            ))
        || (skills.career >= 2.5
            && story.unlock(
                "career",
                "A Door Opens",
                "Your effort is finally being noticed. Bigger opportunities may be ahead.",
            ))
        || (housing.has_access()
            && story.unlock(
                "home",
                "Room to Breathe",
                "You finally have a place to shut the door and call your own.",
            ))
        || (transport.kind.is_vehicle()
            && story.unlock(
                "wheels",
                "The Map Shrinks",
                "With wheels under you, the city suddenly feels smaller and possibility feels closer.",
            ))
        || ((conds.burnout || conds.malnourished || (stats.stress > 85. && stats.energy < 15.))
            && story.unlock(
                "strain",
                "Running on Empty",
                "You can push through anything for a while, but every life asks for balance in the end.",
            ))
        || ((rep.score >= 60. || stats.savings >= 250.)
            && story.unlock(
                "reputation",
                "The Neighborhood Notices",
                "People are starting to recognize your name. Your choices carry more weight now.",
            ))
        || ((rating.score >= 75. || skills.career >= 5.0 || *housing == HousingTier::Penthouse)
            && story.unlock(
                "legacy",
                "More Than Survival",
                "This is no longer just survival. Bit by bit, you are building a life with shape and meaning.",
            ));

    if unlocked && notif.timer <= 0. {
        if gt.day == 0 {
            notif.message = format!(
                "Story: {} - [NE]Office  [E]Shop/Cafe  [SW]Bank  [N]Park+shelter",
                story.current_title
            );
            notif.timer = 9.;
        } else {
            notif.message = format!("Story: {}", story.current_title);
            notif.timer = 5.;
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    fn build_app(day: u32) -> App {
        let mut app = App::new();
        let mut gt = GameTime::default();
        gt.day = day;
        app.insert_resource(gt);
        app.insert_resource(GameState::default());
        app.insert_resource(NpcFriendship::default());
        app.insert_resource(LifeRating::default());
        app.insert_resource(Conditions::default());
        app.insert_resource(Reputation::default());
        app.insert_resource(Transport::default());
        app.insert_resource(NarrativeState::default());
        app.insert_resource(Notification::default());
        app.add_systems(Update, update_narrative);
        app
    }

    fn spawn_player(
        app: &mut App,
        stats: PlayerStats,
        skills: Skills,
        housing: HousingTier,
    ) {
        app.world_mut().spawn((LocalPlayer, stats, skills, housing));
    }

    #[test]
    fn no_player_entity_is_a_noop() {
        let mut app = build_app(0);
        // No LocalPlayer spawned: should early-return cleanly.
        app.update();
        let story = app.world().resource::<NarrativeState>();
        assert_eq!(story.count(), 0);
    }

    #[test]
    fn intro_unlocks_on_day_zero() {
        let mut app = build_app(0);
        spawn_player(
            &mut app,
            PlayerStats::default(),
            Skills::default(),
            HousingTier::Unhoused,
        );
        app.update();
        let story = app.world().resource::<NarrativeState>();
        assert!(story.unlocked.iter().any(|k| k == "intro"));
        assert_eq!(story.current_title, "New in Town");
        // Day-zero notification includes the directional hint.
        let notif = app.world().resource::<Notification>();
        assert!(notif.message.contains("Office"), "got: {}", notif.message);
    }

    #[test]
    fn home_unlocks_when_player_has_housing_access() {
        let mut app = build_app(2);
        spawn_player(
            &mut app,
            PlayerStats::default(),
            Skills::default(),
            HousingTier::Apartment,
        );
        // The unlock chain short-circuits at one beat per frame. On day 2
        // we need at least two ticks (routine, then home) before `home`
        // appears in the unlocked list.
        for _ in 0..6 {
            app.update();
        }
        let story = app.world().resource::<NarrativeState>();
        assert!(
            story.unlocked.iter().any(|k| k == "home"),
            "expected home beat in {:?}",
            story.unlocked
        );
    }

    #[test]
    fn wheels_unlocks_with_vehicle_transport() {
        let mut app = build_app(5);
        spawn_player(
            &mut app,
            PlayerStats::default(),
            Skills::default(),
            HousingTier::Unhoused,
        );
        // Switch transport to a vehicle.
        app.world_mut().resource_mut::<Transport>().kind = TransportKind::Bike;
        // Tick a few times so the chain has a chance to unlock the wheels beat.
        for _ in 0..6 {
            app.update();
        }
        let story = app.world().resource::<NarrativeState>();
        assert!(
            story.unlocked.iter().any(|k| k == "wheels"),
            "expected wheels beat in {:?}",
            story.unlocked
        );
    }

    #[test]
    fn strain_unlocks_on_burnout() {
        let mut app = build_app(3);
        spawn_player(
            &mut app,
            PlayerStats::default(),
            Skills::default(),
            HousingTier::Unhoused,
        );
        app.world_mut().resource_mut::<Conditions>().burnout = true;
        for _ in 0..6 {
            app.update();
        }
        let story = app.world().resource::<NarrativeState>();
        assert!(
            story.unlocked.iter().any(|k| k == "strain"),
            "expected strain beat in {:?}",
            story.unlocked
        );
    }

    #[test]
    fn legacy_unlocks_on_high_rating() {
        let mut app = build_app(8);
        spawn_player(
            &mut app,
            PlayerStats::default(),
            Skills::default(),
            HousingTier::Unhoused,
        );
        app.world_mut().resource_mut::<LifeRating>().score = 80.0;
        for _ in 0..8 {
            app.update();
        }
        let story = app.world().resource::<NarrativeState>();
        assert!(
            story.unlocked.iter().any(|k| k == "legacy"),
            "expected legacy beat in {:?}",
            story.unlocked
        );
    }

    #[test]
    fn already_unlocked_beats_are_not_repeated() {
        let mut app = build_app(0);
        spawn_player(
            &mut app,
            PlayerStats::default(),
            Skills::default(),
            HousingTier::Unhoused,
        );
        app.update();
        let count_after_first = app.world().resource::<NarrativeState>().count();
        // Drain notification timer so a duplicate would have been written.
        app.world_mut().resource_mut::<Notification>().timer = 0.;
        app.world_mut().resource_mut::<Notification>().message = String::new();
        app.update();
        let count_after_second = app.world().resource::<NarrativeState>().count();
        assert_eq!(count_after_first, count_after_second);
    }
}
