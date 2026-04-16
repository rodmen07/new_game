use bevy::prelude::*;
use crate::resources::*;

pub fn check_daily_goal(
    mut goal: ResMut<DailyGoal>,
    gs: Res<GameState>,
    stats: Res<PlayerStats>,
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
    if goal.completed || goal.failed { return; }

    let (raw_progress, done): (f32, bool) = match &goal.kind {
        GoalKind::EarnMoney     => (gs.money_earned_today, gs.money_earned_today >= goal.target),
        GoalKind::WorkTimes     => (gs.work_today as f32, gs.work_today as f32 >= goal.target),
        GoalKind::MaintainHappy => (stats.happiness, stats.happiness >= 60.),
        GoalKind::EatTimes      => (gs.eat_today as f32, gs.eat_today as f32 >= goal.target),
        GoalKind::ChatTimes     => (gs.chat_today as f32, gs.chat_today as f32 >= goal.target),
        GoalKind::FriendNpc     => {
            let best = friendship.levels.values().cloned().fold(0f32, f32::max);
            (best, best >= goal.target)
        },
        GoalKind::SaveMoney     => (stats.savings, stats.savings >= goal.target),
        GoalKind::ExerciseTimes => (gs.exercise_today as f32, gs.exercise_today as f32 >= goal.target),
        GoalKind::LowerStress   => (100. - stats.stress, stats.stress <= goal.target),
        GoalKind::BuildStreak   => (streak.days as f32, streak.days as f32 >= goal.target),
        GoalKind::MasterHobby   => {
            let best = hobbies.best();
            (best, best >= goal.target)
        },
        GoalKind::EarnPassive   => (gs.passive_income_today, gs.passive_income_today >= goal.target),
        GoalKind::OutdoorWeather => {
            let did = gs.outdoor_done_today && weather.is_sunny();
            (if did { 1. } else { 0. }, did)
        },
        GoalKind::StudyTimes    => (gs.study_today as f32, gs.study_today as f32 >= goal.target),
        GoalKind::FeedPet       => {
            let done = pet.has_pet && pet.fed_today;
            (if done { 1. } else if pet.has_pet { 0.5 } else { 0. }, done)
        },
        GoalKind::ThrowParty    => {
            let done = social_events.party_today;
            (if done { 1. } else { 0. }, done)
        },
        GoalKind::OwnVehicle    => {
            let owned = transport.kind.is_vehicle();
            (if owned { 1. } else { 0. }, owned)
        },
        GoalKind::SeasonalGoal  => match &season.current {
            SeasonKind::Spring => (gs.exercise_today as f32, gs.exercise_today >= 2),
            SeasonKind::Summer => (if gs.outdoor_done_today { 1. } else { 0. }, gs.outdoor_done_today),
            SeasonKind::Autumn => (gs.passive_income_today, gs.passive_income_today >= goal.target),
            SeasonKind::Winter => (gs.hobby_today as f32, gs.hobby_today >= 2),
        },
    };

    // ThrowParty progress is handled correctly via social_events.party_today above
    goal.progress = raw_progress;
    if done && !goal.completed {
        goal.completed = true;
        if notif.timer <= 0. {
            notif.message = format!("Goal complete! +${:.0} +{}hap", goal.reward_money, goal.reward_happiness as i32);
            notif.timer = 5.;
        }
    }
    if stats.happiness < 60. && matches!(&goal.kind, GoalKind::MaintainHappy) { goal.failed = true; }
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
) {
    macro_rules! unlock {
        ($flag:expr, $name:expr) => {
            if !$flag {
                $flag = true;
                if notif.timer <= 0. {
                    notif.message = format!("Milestone: {}! ({}/15)", $name, ms.count());
                    notif.timer = 6.;
                }
            }
        };
    }

    if stats.savings >= 500.                                     { unlock!(ms.saved_100,    "Saver — $500 saved");          }
    if stats.health >= 90. && stats.happiness >= 90. && stats.energy >= 90. {
                                                                   unlock!(ms.rating_a,     "Thriving — Peak wellness");    }
    if skills.career >= 5.                                       { unlock!(ms.exec,         "Executive");                   }
    if streak.days >= 7                                          { unlock!(ms.streak_7,     "Streak 7 days");               }
    if stats.loan == 0. && gs.days_survived > 5                  { unlock!(ms.debt_free,    "Debt Free");                   }
    if friendship.levels.values().any(|&v| v >= 5.)              { unlock!(ms.best_friend,  "Best Friends Forever");        }
    if *housing == HousingTier::Penthouse                           { unlock!(ms.penthouse,    "Penthouse Owner");             }
    if invest.total_return >= 100.                               { unlock!(ms.investor,     "Investor — $100 return");      }
    if hobbies.best() >= 5.                                      { unlock!(ms.hobbyist,     "Hobbyist — Max hobby");        }
    if rep.score >= 80.                                          { unlock!(ms.famous,       "Famous — Rep 80+");            }
    if gs.study_today >= 2 || (skills.cooking + skills.career + skills.fitness + skills.social) >= 12. {
                                                                   unlock!(ms.scholar,      "Scholar — Skills 12 total");   }
    if pet.has_pet                                               { unlock!(ms.pet_owner,    "Pet Owner");                   }
    if social_events.parties_thrown >= 5                         { unlock!(ms.party_animal, "Party Animal — 5 parties");   }
    if transport.kind.is_vehicle()                               { unlock!(ms.commuter,     "Commuter — Own a vehicle");    }
    if gt.day >= 120                                             { unlock!(ms.all_seasons,  "All Seasons — 120 days");      }
}