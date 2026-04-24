use crate::resources::*;
use bevy::prelude::*;

/// Checks once per day whether a festival should start or end.
/// Festivals run on days 25-27 of each 30-day season.
pub fn festival_trigger_system(
    gt: Res<GameTime>,
    mut festival: ResMut<FestivalState>,
    season: Res<Season>,
    mut notif: ResMut<Notification>,
) {
    if gt.day == festival.last_check_day {
        return;
    }
    festival.last_check_day = gt.day;
    festival.activities_today = 0;

    let season_day = gt.day % 30;

    // Festival runs on days 25, 26, 27 of each season
    if (25..=27).contains(&season_day) {
        let kind = match &season.current {
            SeasonKind::Spring => FestivalKind::SpringFair,
            SeasonKind::Summer => FestivalKind::SummerBBQ,
            SeasonKind::Autumn => FestivalKind::AutumnHarvest,
            SeasonKind::Winter => FestivalKind::WinterGala,
        };

        if festival.active.is_none() {
            notif.push(
                format!(
                    "{} has begun! Visit the Park for special activities.",
                    kind.label()
                ),
                6.,
            );
        }

        festival.active = Some(kind);
    } else if let Some(kind) = festival.active.take() {
        notif.push(
            format!(
                "{} is over! You earned {} tokens total.",
                kind.label(),
                festival.tokens
            ),
            5.,
        );
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    fn run_with(
        day: u32,
        season: SeasonKind,
        mut state: FestivalState,
    ) -> (FestivalState, Notification) {
        let mut app = App::new();
        let mut gt = GameTime::default();
        gt.day = day;
        // Force last_check_day to differ so the system runs its body.
        if state.last_check_day == day {
            state.last_check_day = day.wrapping_sub(1);
        }
        app.insert_resource(gt);
        app.insert_resource(state);
        app.insert_resource(Season { current: season });
        app.insert_resource(Notification::default());
        app.add_systems(Update, festival_trigger_system);
        app.update();
        let festival = app.world_mut().remove_resource::<FestivalState>().unwrap();
        let notif = app.world_mut().remove_resource::<Notification>().unwrap();
        (festival, notif)
    }

    #[test]
    fn no_festival_outside_window() {
        let (f, n) = run_with(10, SeasonKind::Spring, FestivalState::default());
        assert!(!f.is_active());
        assert!(n.message.is_empty(), "no announcement on regular days");
        assert_eq!(f.last_check_day, 10);
    }

    #[test]
    fn festival_starts_on_day_25_of_season() {
        // Day 25 (season day 25) → festival begins.
        let (f, n) = run_with(25, SeasonKind::Spring, FestivalState::default());
        assert!(f.is_active());
        assert!(matches!(f.active, Some(FestivalKind::SpringFair)));
        assert!(n.message.contains("Spring Fair"), "got: {}", n.message);
    }

    #[test]
    fn festival_kind_matches_current_season() {
        // Same window, different seasons → different kinds.
        let cases = [
            (SeasonKind::Spring, FestivalKind::SpringFair, "Spring Fair"),
            (SeasonKind::Summer, FestivalKind::SummerBBQ, "Summer BBQ"),
            (
                SeasonKind::Autumn,
                FestivalKind::AutumnHarvest,
                "Autumn Harvest",
            ),
            (SeasonKind::Winter, FestivalKind::WinterGala, "Winter Gala"),
        ];
        for (season, expected_kind, label) in cases {
            let (f, n) = run_with(26, season, FestivalState::default());
            assert!(f.is_active());
            assert!(
                f.active.as_ref().unwrap() == &expected_kind,
                "expected {} festival",
                label
            );
            assert!(n.message.contains(label));
        }
    }

    #[test]
    fn festival_does_not_reannounce_on_subsequent_days() {
        // Day 25 → already started. Re-running on day 26 should NOT push a new "begun" notification.
        let mut state = FestivalState::default();
        state.active = Some(FestivalKind::SpringFair);
        state.last_check_day = 25;
        let (f, n) = run_with(26, SeasonKind::Spring, state);
        assert!(f.is_active());
        assert!(
            n.message.is_empty(),
            "no duplicate announcement, got: {}",
            n.message
        );
    }

    #[test]
    fn festival_ends_after_window() {
        // Day 28 (just past day 27 end) should clear an active festival
        // and announce "is over".
        let mut state = FestivalState::default();
        state.active = Some(FestivalKind::SummerBBQ);
        state.tokens = 12;
        state.last_check_day = 27;
        let (f, n) = run_with(28, SeasonKind::Summer, state);
        assert!(!f.is_active());
        assert!(n.message.contains("is over"), "got: {}", n.message);
        assert!(n.message.contains("12 tokens"), "got: {}", n.message);
    }

    #[test]
    fn early_return_when_already_checked_today() {
        // last_check_day == current day → no work done, activities_today preserved.
        let mut state = FestivalState::default();
        state.last_check_day = 5;
        state.activities_today = 3;
        let (f, _) = run_with_no_advance(5, SeasonKind::Spring, state);
        assert_eq!(f.activities_today, 3, "should not be reset");
    }

    fn run_with_no_advance(
        day: u32,
        season: SeasonKind,
        state: FestivalState,
    ) -> (FestivalState, Notification) {
        let mut app = App::new();
        let mut gt = GameTime::default();
        gt.day = day;
        app.insert_resource(gt);
        app.insert_resource(state);
        app.insert_resource(Season { current: season });
        app.insert_resource(Notification::default());
        app.add_systems(Update, festival_trigger_system);
        app.update();
        let festival = app.world_mut().remove_resource::<FestivalState>().unwrap();
        let notif = app.world_mut().remove_resource::<Notification>().unwrap();
        (festival, notif)
    }

    #[test]
    fn festival_resets_activities_today_on_new_day() {
        let mut state = FestivalState::default();
        state.activities_today = 7;
        state.last_check_day = 0;
        let (f, _) = run_with(1, SeasonKind::Spring, state);
        assert_eq!(f.activities_today, 0);
    }
}
