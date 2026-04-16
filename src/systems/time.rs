use bevy::prelude::*;
use crate::constants::TIME_SCALE;
use crate::resources::*;

pub fn tick_time(mut gt: ResMut<GameTime>, time: Res<Time>) {
    gt.hours += time.delta_secs() * TIME_SCALE / 3600.;
    if gt.hours >= 24. { gt.hours -= 24.; gt.day += 1; }
}

pub fn on_new_day(
    mut gt: ResMut<GameTime>, mut stats: ResMut<PlayerStats>, mut gs: ResMut<GameState>,
    mut goal: ResMut<DailyGoal>, mut notif: ResMut<Notification>, mut rating: ResMut<LifeRating>,
    mut streak: ResMut<WorkStreak>, mut friendship: ResMut<NpcFriendship>,
    housing: Res<HousingTier>, skills: Res<Skills>,
    mut weather: ResMut<WeatherKind>, hobbies: Res<Hobbies>,
    mut conds: ResMut<Conditions>, mut invest: ResMut<Investment>, mut rep: ResMut<Reputation>,
    mut day_extras: DayExtras,
) {
    if gt.day == gt.prev_day { return; }
    gt.prev_day = gt.day;

    rating.sample(&stats, &skills);

    // ── Season update ─────────────────────────────────────────────────────────
    day_extras.season.current = SeasonKind::from_day(gt.day);

    // ── Weather update ────────────────────────────────────────────────────────
    *weather = WeatherKind::from_day(gt.day);
    if weather.is_stormy() && notif.timer <= 0. {
        notif.message = format!("Day {} — Stormy! Stay indoors.", gt.day + 1);
        notif.timer = 4.;
    }

    // ── Condition resolution ──────────────────────────────────────────────────
    if gs.high_stress_today {
        conds.burnout_days += 1;
        if conds.burnout_days >= 3 && !conds.burnout {
            conds.burnout = true;
            if notif.timer <= 0. { notif.message = "Burnout! Work pay -30%. Rest up.".into(); notif.timer = 6.; }
        }
    } else if stats.stress < 40. {
        if conds.burnout_days > 0 { conds.burnout_days -= 1; }
        if conds.burnout_days == 0 { conds.burnout = false; }
    }
    if gs.high_hunger_today {
        conds.malnourish_days += 1;
        if conds.malnourish_days >= 3 && !conds.malnourished {
            conds.malnourished = true;
            if notif.timer <= 0. { notif.message = "Malnourished! Health drains faster.".into(); notif.timer = 6.; }
        }
    } else if stats.hunger < 40. {
        // Eating well — recover from malnourishment track
        if conds.malnourish_days > 0 { conds.malnourish_days -= 1; }
        if conds.malnourish_days == 0 { conds.malnourished = false; }
    }

    // ── Pet hunger daily tick ─────────────────────────────────────────────────
    if day_extras.pet.has_pet {
        if !day_extras.pet.fed_today {
            day_extras.pet.hunger = (day_extras.pet.hunger + 30.).min(100.);
            stats.happiness = (stats.happiness - 10.).max(0.);
            if notif.timer <= 0. { notif.message = format!("{} is hungry! Feed your pet.", day_extras.pet.name); notif.timer = 4.; }
        } else {
            stats.happiness = (stats.happiness + 5.).min(100.);
        }
        day_extras.pet.fed_today = false;
    }

    // ── Passive hobby income (× Autumn bonus) ────────────────────────────────
    let passive = hobbies.passive_income() * day_extras.season.current.passive_mult();
    if passive > 0. {
        stats.money += passive;
        gs.passive_income_today += passive;
        if notif.timer <= 0. {
            notif.message = format!("Passive income: +${:.0} from hobbies!", passive);
            notif.timer = 3.;
        }
    }

    // ── Investment daily return ───────────────────────────────────────────────
    if invest.amount > 0. && invest.risk > 0 {
        let seed = gt.day.wrapping_mul(1664525).wrapping_add(invest.risk as u32 * 999983);
        let rand_sign: f32 = if seed % 2 == 0 { 1.0 } else { -1.0 };
        let rand_frac: f32 = (seed % 1000) as f32 / 1000.0;
        let daily_rate = invest.daily_return_rate * rand_sign * (0.5 + rand_frac * 0.5);
        let change = invest.amount * daily_rate;
        invest.amount = (invest.amount + change).max(0.);
        invest.total_return += change;
        if change.abs() > invest.amount * 0.10 && notif.timer <= 0. {
            let dir = if change > 0. { "gained" } else { "lost" };
            notif.message = format!("Investment {} ${:.0}! Portfolio: ${:.0}", dir, change.abs(), invest.amount);
            notif.timer = 4.;
        }
    }

    // ── Reputation decay ──────────────────────────────────────────────────────
    rep.score = (rep.score - 1.0).max(0.);

    // ── Loan daily interest 8% ────────────────────────────────────────────────
    if stats.loan > 0. {
        stats.loan *= 1.08;
        stats.stress = (stats.stress + 5.).min(100.);
        if stats.loan > 300. && notif.timer <= 0. {
            notif.message = format!("Loan is now ${:.0}! Pay it off at the Bank.", stats.loan);
            notif.timer = 5.;
        }
    }

    // ── Savings 5% interest ───────────────────────────────────────────────────
    if stats.savings > 0. { stats.savings *= 1.05; }

    // ── Sleep debt accumulates 8h per day ────────────────────────────────────
    stats.sleep_debt = (stats.sleep_debt + 8.).min(32.);

    // ── Season indoor/outdoor bonus at day start ──────────────────────────────
    let winter_bonus = day_extras.season.current.indoor_bonus();
    if winter_bonus > 0. { stats.happiness = (stats.happiness + winter_bonus).min(100.); }

    // ── Housing morning bonus ─────────────────────────────────────────────────
    let hap_bonus = housing.morning_hap();
    if hap_bonus > 0. { stats.happiness = (stats.happiness + hap_bonus).min(100.); }

    // ── Rent ──────────────────────────────────────────────────────────────────
    let rent = housing.rent();
    if stats.money >= rent {
        stats.money -= rent;
        stats.unpaid_rent_days = 0;
    } else {
        stats.unpaid_rent_days += 1;
        stats.happiness = (stats.happiness - 20.).max(0.);
        stats.stress = (stats.stress + 20.).min(100.);
        if stats.unpaid_rent_days >= 3 {
            stats.money = 0.; stats.health = (stats.health - 20.).max(0.);
            stats.unpaid_rent_days = 0;
            notif.message = "Evicted! Get to work immediately.".into();
            notif.timer = 7.;
        } else if notif.timer <= 0. {
            notif.message = format!("Cannot pay rent (${:.0})! {} day(s) unpaid.", rent, stats.unpaid_rent_days);
            notif.timer = 6.;
        }
    }

    // ── Work streak reset ─────────────────────────────────────────────────────
    if !streak.worked_today && streak.days > 0 {
        streak.days = 0;
        if notif.timer <= 0. { notif.message = "Work streak broken!".into(); notif.timer = 3.; }
    }
    streak.worked_today = false;

    // ── Friendship decay ──────────────────────────────────────────────────────
    let not_chatted: Vec<Entity> = friendship.levels.keys()
        .filter(|e| !friendship.chatted_today.get(*e).copied().unwrap_or(false))
        .cloned().collect();
    for e in not_chatted {
        if let Some(lvl) = friendship.levels.get_mut(&e) { *lvl = (*lvl - 0.15).max(0.); }
    }
    friendship.chatted_today.clear();

    // ── Reset daily state ─────────────────────────────────────────────────────
    day_extras.social_events.party_today = false;
    gs.days_survived = gt.day;
    gs.money_earned_today = 0.; gs.work_today = 0; gs.eat_today = 0;
    gs.chat_today = 0; gs.exercise_today = 0; gs.hobby_today = 0;
    gs.passive_income_today = 0.; gs.outdoor_done_today = false; gs.study_today = 0;
    gs.high_stress_today = false; gs.high_hunger_today = false;
    *goal = make_goal(gt.day, &day_extras.season.current);

    if gt.is_weekend() && notif.timer <= 0. {
        notif.message = format!("It''s {}! Work pays 1.5x today.", gt.day_name());
        notif.timer = 4.;
        return;
    }

    type Eff = fn(&mut PlayerStats);
    let events: &[(&str, Eff)] = &[
        ("Payday bonus! +$50",          |s| s.money += 50.),
        ("Unexpected bill! -$35",       |s| s.money = (s.money-35.).max(0.)),
        ("Well rested! +Energy",        |s| s.energy = (s.energy+15.).min(100.)),
        ("Noisy night. -Energy",        |s| s.energy = (s.energy-15.).max(0.)),
        ("Found $20!",                  |s| s.money += 20.),
        ("Car repair. -$45",            |s| s.money = (s.money-45.).max(0.)),
        ("Beautiful morning!",          |s| s.happiness = (s.happiness+15.).min(100.)),
        ("Feeling ill. -Energy -Mood",  |s| { s.energy=(s.energy-10.).max(0.); s.happiness=(s.happiness-10.).max(0.); }),
        ("Kind message! +Happiness",    |s| s.happiness = (s.happiness+12.).min(100.)),
        ("Subscription fee. -$20",      |s| s.money = (s.money-20.).max(0.)),
        ("Doctor visit. +15 Health",    |s| s.health = (s.health+15.).min(100.)),
        ("Bad meal. -10 Health",        |s| s.health = (s.health-10.).max(0.)),
        ("Tax refund! +$30",            |s| s.money += 30.),
        ("Work relief. -Stress",        |s| s.stress = (s.stress-15.).max(0.)),
    ];
    let idx = (gt.day as usize).wrapping_mul(2654435761) % events.len();
    let (msg, eff) = events[idx];
    eff(&mut stats);
    if notif.timer <= 0. { notif.message = format!("Day {} -- {}", gt.day+1, msg); notif.timer = 5.; }
}

pub fn make_goal(day: u32, season: &SeasonKind) -> DailyGoal {
    let (seasonal_desc, seasonal_target) = season.seasonal_goal_desc();
    match day % 18 {
        0  => DailyGoal { kind: GoalKind::EarnMoney,     description: "Earn $50 today".into(),               progress:0.,target:50., reward_money:25.,reward_happiness:0., completed:false,failed:false },
        1  => DailyGoal { kind: GoalKind::WorkTimes,     description: "Work 3 times today".into(),            progress:0.,target:3.,  reward_money:30.,reward_happiness:0., completed:false,failed:false },
        2  => DailyGoal { kind: GoalKind::MaintainHappy, description: "Stay happy (60%+) all day".into(),     progress:0.,target:1.,  reward_money:15.,reward_happiness:10.,completed:false,failed:false },
        3  => DailyGoal { kind: GoalKind::EatTimes,      description: "Eat 2 meals today".into(),             progress:0.,target:2.,  reward_money:0., reward_happiness:20.,completed:false,failed:false },
        4  => DailyGoal { kind: GoalKind::ChatTimes,     description: "Chat with 2 people".into(),            progress:0.,target:2.,  reward_money:10.,reward_happiness:15.,completed:false,failed:false },
        5  => DailyGoal { kind: GoalKind::FriendNpc,     description: "Friendship 2 with any NPC".into(),     progress:0.,target:2.,  reward_money:20.,reward_happiness:10.,completed:false,failed:false },
        6  => DailyGoal { kind: GoalKind::SaveMoney,     description: "Have $30+ in savings".into(),          progress:0.,target:30., reward_money:15.,reward_happiness:0., completed:false,failed:false },
        7  => DailyGoal { kind: GoalKind::ExerciseTimes, description: "Exercise 2 times today".into(),        progress:0.,target:2.,  reward_money:0., reward_happiness:20.,completed:false,failed:false },
        8  => DailyGoal { kind: GoalKind::LowerStress,   description: "Get stress below 30".into(),           progress:0.,target:30., reward_money:20.,reward_happiness:10.,completed:false,failed:false },
        9  => DailyGoal { kind: GoalKind::BuildStreak,   description: "Maintain a 3-day work streak".into(),  progress:0.,target:3.,  reward_money:35.,reward_happiness:5., completed:false,failed:false },
        10 => DailyGoal { kind: GoalKind::MasterHobby,   description: "Reach level 3 in any hobby".into(),    progress:0.,target:3.,  reward_money:30.,reward_happiness:0., completed:false,failed:false },
        11 => DailyGoal { kind: GoalKind::EarnPassive,   description: "Earn $10 passive income today".into(), progress:0.,target:10., reward_money:25.,reward_happiness:0., completed:false,failed:false },
        12 => DailyGoal { kind: GoalKind::OutdoorWeather,description: "Go outside on a Sunny day".into(),     progress:0.,target:1.,  reward_money:20.,reward_happiness:15.,completed:false,failed:false },
        13 => DailyGoal { kind: GoalKind::StudyTimes,    description: "Study 2 times today ($30 ea)".into(),  progress:0.,target:2.,  reward_money:20.,reward_happiness:5., completed:false,failed:false },
        14 => DailyGoal { kind: GoalKind::FeedPet,       description: "Feed your pet today".into(),           progress:0.,target:1.,  reward_money:15.,reward_happiness:20.,completed:false,failed:false },
        15 => DailyGoal { kind: GoalKind::ThrowParty,    description: "Throw a party today ($40)".into(),     progress:0.,target:1.,  reward_money:10.,reward_happiness:30.,completed:false,failed:false },
        16 => DailyGoal { kind: GoalKind::OwnVehicle,    description: "Own a Bike or Car".into(),             progress:0.,target:1.,  reward_money:30.,reward_happiness:0., completed:false,failed:false },
        _  => DailyGoal { kind: GoalKind::SeasonalGoal,  description: seasonal_desc.into(),                   progress:0.,target:seasonal_target, reward_money:25.,reward_happiness:10.,completed:false,failed:false },
    }
}