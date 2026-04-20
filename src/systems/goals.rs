#![allow(clippy::too_many_arguments)]

use crate::resources::*;
use bevy::prelude::*;

/// Pure computation: given a goal kind, target value, and the relevant snapshot
/// values, returns (raw_progress, done).
pub fn compute_goal_progress(
    kind: &GoalKind,
    target: f32,
    happiness: f32,
    money_earned: f32,
    work_today: u32,
    eat_today: u32,
    chat_today: u32,
    best_friendship: f32,
    savings: f32,
    money: f32,
    exercise_today: u32,
    stress: f32,
    streak_days: u32,
    best_hobby: f32,
    passive_income: f32,
    outdoor_done: bool,
    is_sunny: bool,
    study_today: u32,
    has_pet: bool,
    fed_pet: bool,
    party_today: bool,
    owns_vehicle: bool,
    season: &SeasonKind,
    hobby_today: u32,
) -> (f32, bool) {
    match kind {
        GoalKind::EarnMoney => (money_earned, money_earned >= target),
        GoalKind::WorkTimes => (work_today as f32, work_today as f32 >= target),
        GoalKind::MaintainHappy => (happiness, happiness >= 60.),
        GoalKind::EatTimes => (eat_today as f32, eat_today as f32 >= target),
        GoalKind::ChatTimes => (chat_today as f32, chat_today as f32 >= target),
        GoalKind::FriendNpc => (best_friendship, best_friendship >= target),
        GoalKind::SaveMoney => {
            let potential = (savings + money).min(target);
            (potential, savings >= target)
        }
        GoalKind::ExerciseTimes => (exercise_today as f32, exercise_today as f32 >= target),
        GoalKind::LowerStress => (100. - stress, stress <= target),
        GoalKind::BuildStreak => (streak_days as f32, streak_days as f32 >= target),
        GoalKind::MasterHobby => (best_hobby, best_hobby >= target),
        GoalKind::EarnPassive => (passive_income, passive_income >= target),
        GoalKind::OutdoorWeather => {
            let did = outdoor_done && is_sunny;
            (if did { 1. } else { 0. }, did)
        }
        GoalKind::StudyTimes => (study_today as f32, study_today as f32 >= target),
        GoalKind::FeedPet => {
            let done = has_pet && fed_pet;
            (
                if done {
                    1.
                } else if has_pet {
                    0.5
                } else {
                    0.
                },
                done,
            )
        }
        GoalKind::ThrowParty => (if party_today { 1. } else { 0. }, party_today),
        GoalKind::OwnVehicle => (if owns_vehicle { 1. } else { 0. }, owns_vehicle),
        GoalKind::SeasonalGoal => match season {
            SeasonKind::Spring => (exercise_today as f32, exercise_today >= 2),
            SeasonKind::Summer => (if outdoor_done { 1. } else { 0. }, outdoor_done),
            SeasonKind::Autumn => (passive_income, passive_income >= target),
            SeasonKind::Winter => (hobby_today as f32, hobby_today >= 2),
        },
    }
}

pub fn check_daily_goal(
    mut goal: ResMut<DailyGoal>,
    gs: Res<GameState>,
    mut stats: ResMut<PlayerStats>,
    streak: Res<WorkStreak>,
    friendship: Res<NpcFriendship>,
    mut notif: ResMut<Notification>,
    hobbies: Res<Hobbies>,
    weather: Res<WeatherKind>,
    pet: Res<Pet>,
    transport: Res<Transport>,
    season: Res<Season>,
    social_events: Res<SocialEvents>,
) {
    if goal.completed || goal.failed {
        return;
    }

    let best_friendship = friendship.levels.values().cloned().fold(0f32, f32::max);
    let (raw_progress, done) = compute_goal_progress(
        &goal.kind,
        goal.target,
        stats.happiness,
        gs.money_earned_today,
        gs.work_today,
        gs.eat_today,
        gs.chat_today,
        best_friendship,
        stats.savings,
        stats.money,
        gs.exercise_today,
        stats.stress,
        streak.days,
        hobbies.best(),
        gs.passive_income_today,
        gs.outdoor_done_today,
        weather.is_sunny(),
        gs.study_today,
        pet.has_pet,
        pet.fed_today,
        social_events.party_today,
        transport.kind.is_vehicle(),
        &season.current,
        gs.hobby_today,
    );

    goal.progress = raw_progress;
    if done && !goal.completed {
        goal.completed = true;
        stats.money += goal.reward_money;
        stats.happiness = (stats.happiness + goal.reward_happiness).min(100.);
        notif.push(
            format!(
                "Goal complete! +${:.0} +{}hap",
                goal.reward_money, goal.reward_happiness as i32
            ),
            5.,
        );
    }
    if stats.happiness < 60. && matches!(&goal.kind, GoalKind::MaintainHappy) {
        goal.failed = true;
    }
}

pub fn check_milestones(
    mut ms: ResMut<Milestones>,
    stats: Res<PlayerStats>,
    gs: Res<GameState>,
    skills: Res<Skills>,
    streak: Res<WorkStreak>,
    gt: Res<GameTime>,
    friendship: Res<NpcFriendship>,
    invest: Res<Investment>,
    hobbies: Res<Hobbies>,
    rep: Res<Reputation>,
    pet: Res<Pet>,
    social_events: Res<SocialEvents>,
    transport: Res<Transport>,
    housing: Res<HousingTier>,
    mut notif: ResMut<Notification>,
    ms_extras: MilestoneExtras,
) {
    macro_rules! unlock {
        ($flag:expr, $name:expr) => {
            if !$flag {
                $flag = true;
                notif.push(format!("Milestone: {}! ({}/21)", $name, ms.count()), 6.);
            }
        };
    }

    if stats.savings >= 500. {
        unlock!(ms.saved_100, "Saver — $500 saved");
    }
    if stats.health >= 90. && stats.happiness >= 90. && stats.energy >= 90. {
        unlock!(ms.rating_a, "Thriving — Peak wellness");
    }
    if skills.career >= 5. {
        unlock!(ms.exec, "Executive");
    }
    if streak.days >= 7 {
        unlock!(ms.streak_7, "Streak 7 days");
    }
    if stats.loan == 0. && gs.days_survived > 5 {
        unlock!(ms.debt_free, "Debt Free");
    }
    if friendship.levels.values().any(|&v| v >= 5.) {
        unlock!(ms.best_friend, "Best Friends Forever");
    }
    if *housing == HousingTier::Penthouse {
        unlock!(ms.penthouse, "Penthouse Owner");
    }
    if invest.total_return >= 100. {
        unlock!(ms.investor, "Investor — $100 return");
    }
    if hobbies.best() >= 5. {
        unlock!(ms.hobbyist, "Hobbyist — Max hobby");
    }
    if rep.score >= 80. {
        unlock!(ms.famous, "Famous — Rep 80+");
    }
    if gs.study_today >= 2
        || (skills.cooking + skills.career + skills.fitness + skills.social) >= 12.
    {
        unlock!(ms.scholar, "Scholar — Skills 12 total");
    }
    if pet.has_pet {
        unlock!(ms.pet_owner, "Pet Owner");
    }
    if social_events.parties_thrown >= 5 {
        unlock!(ms.party_animal, "Party Animal — 5 parties");
    }
    if transport.kind.is_vehicle() {
        unlock!(ms.commuter, "Commuter — Own a vehicle");
    }
    if gt.day >= 120 {
        unlock!(ms.all_seasons, "All Seasons — 120 days");
    }
    if gs.total_quests >= 10 {
        unlock!(ms.quest_master, "Quest Master — 10 quests");
    }
    if gs.total_crafted >= 20 {
        unlock!(ms.master_chef, "Master Chef — 20 crafts");
    }
    if gs.total_gifts >= 10 {
        unlock!(ms.gift_giver, "Gift Giver — 10 gifts");
    }
    if friendship.levels.values().filter(|&&v| v >= 2.).count() >= 6 {
        unlock!(ms.popular, "Popular - All NPCs at Friend+");
    }
    if ms_extras.crisis.crises_survived >= 5 {
        unlock!(ms.crisis_survivor, "Survivor - 5 crises survived");
    }
    if ms_extras.festival.all_seasons_attended() {
        unlock!(ms.festival_goer, "Festival Goer - all 4 seasonal festivals");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper with sensible defaults; override fields as needed.
    fn progress(
        kind: GoalKind,
        target: f32,
        overrides: impl FnOnce(&mut GoalInputs),
    ) -> (f32, bool) {
        let mut i = GoalInputs::default();
        i.target = target;
        overrides(&mut i);
        compute_goal_progress(
            &kind,
            i.target,
            i.happiness,
            i.money_earned,
            i.work_today,
            i.eat_today,
            i.chat_today,
            i.best_friendship,
            i.savings,
            i.money,
            i.exercise_today,
            i.stress,
            i.streak_days,
            i.best_hobby,
            i.passive_income,
            i.outdoor_done,
            i.is_sunny,
            i.study_today,
            i.has_pet,
            i.fed_pet,
            i.party_today,
            i.owns_vehicle,
            &i.season,
            i.hobby_today,
        )
    }

    #[derive(Default)]
    struct GoalInputs {
        target: f32,
        happiness: f32,
        money_earned: f32,
        work_today: u32,
        eat_today: u32,
        chat_today: u32,
        best_friendship: f32,
        savings: f32,
        money: f32,
        exercise_today: u32,
        stress: f32,
        streak_days: u32,
        best_hobby: f32,
        passive_income: f32,
        outdoor_done: bool,
        is_sunny: bool,
        study_today: u32,
        has_pet: bool,
        fed_pet: bool,
        party_today: bool,
        owns_vehicle: bool,
        season: SeasonKind,
        hobby_today: u32,
    }

    #[test]
    fn earn_money_incomplete() {
        let (prog, done) = progress(GoalKind::EarnMoney, 50., |i| i.money_earned = 30.);
        assert!((prog - 30.).abs() < f32::EPSILON);
        assert!(!done);
    }

    #[test]
    fn earn_money_complete() {
        let (prog, done) = progress(GoalKind::EarnMoney, 50., |i| i.money_earned = 50.);
        assert!((prog - 50.).abs() < f32::EPSILON);
        assert!(done);
    }

    #[test]
    fn save_money_needs_actual_savings() {
        let (prog, done) = progress(GoalKind::SaveMoney, 100., |i| {
            i.money = 80.;
            i.savings = 20.;
        });
        assert!((prog - 100.).abs() < f32::EPSILON);
        assert!(!done); // savings < target even though cash+savings >= target
    }

    #[test]
    fn save_money_done_when_savings_enough() {
        let (_, done) = progress(GoalKind::SaveMoney, 100., |i| i.savings = 100.);
        assert!(done);
    }

    #[test]
    fn lower_stress_progress_inverted() {
        let (prog, done) = progress(GoalKind::LowerStress, 40., |i| i.stress = 30.);
        assert!((prog - 70.).abs() < f32::EPSILON); // 100 - 30
        assert!(done); // stress 30 <= target 40
    }

    #[test]
    fn lower_stress_not_done() {
        let (_, done) = progress(GoalKind::LowerStress, 40., |i| i.stress = 50.);
        assert!(!done);
    }

    #[test]
    fn feed_pet_no_pet() {
        let (prog, done) = progress(GoalKind::FeedPet, 1., |_| {});
        assert!((prog - 0.).abs() < f32::EPSILON);
        assert!(!done);
    }

    #[test]
    fn feed_pet_has_pet_not_fed() {
        let (prog, done) = progress(GoalKind::FeedPet, 1., |i| i.has_pet = true);
        assert!((prog - 0.5).abs() < f32::EPSILON);
        assert!(!done);
    }

    #[test]
    fn feed_pet_done() {
        let (prog, done) = progress(GoalKind::FeedPet, 1., |i| {
            i.has_pet = true;
            i.fed_pet = true;
        });
        assert!((prog - 1.).abs() < f32::EPSILON);
        assert!(done);
    }

    #[test]
    fn outdoor_weather_needs_sunny() {
        let (_, done) = progress(GoalKind::OutdoorWeather, 1., |i| {
            i.outdoor_done = true;
            i.is_sunny = false;
        });
        assert!(!done);
    }

    #[test]
    fn outdoor_weather_sunny_and_done() {
        let (prog, done) = progress(GoalKind::OutdoorWeather, 1., |i| {
            i.outdoor_done = true;
            i.is_sunny = true;
        });
        assert!((prog - 1.).abs() < f32::EPSILON);
        assert!(done);
    }

    #[test]
    fn seasonal_spring_exercise() {
        let (prog, done) = progress(GoalKind::SeasonalGoal, 2., |i| {
            i.season = SeasonKind::Spring;
            i.exercise_today = 2;
        });
        assert!((prog - 2.).abs() < f32::EPSILON);
        assert!(done);
    }

    #[test]
    fn seasonal_winter_hobby() {
        let (_, done) = progress(GoalKind::SeasonalGoal, 2., |i| {
            i.season = SeasonKind::Winter;
            i.hobby_today = 1;
        });
        assert!(!done); // needs 2
    }

    #[test]
    fn maintain_happy_threshold() {
        let (_, done) = progress(GoalKind::MaintainHappy, 60., |i| i.happiness = 59.);
        assert!(!done);
        let (_, done) = progress(GoalKind::MaintainHappy, 60., |i| i.happiness = 60.);
        assert!(done);
    }

    #[test]
    fn work_times_exact_target() {
        let (prog, done) = progress(GoalKind::WorkTimes, 3., |i| i.work_today = 3);
        assert!((prog - 3.).abs() < f32::EPSILON);
        assert!(done);
    }
}
