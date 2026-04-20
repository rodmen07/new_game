#![allow(clippy::too_many_arguments)]

use crate::audio::{PlaySfx, SfxKind};
use crate::components::{Npc, NpcPersonality, Player};
use crate::constants::TIME_SCALE;
use crate::resources::*;
use bevy::prelude::*;

struct DailyEvt {
    msg: &'static str,
    eff: fn(&mut PlayerStats),
    rep: f32,
    season: u8, // 0=any, 1=Spring, 2=Summer, 3=Autumn, 4=Winter
    cond: u8,   // 0=always, 1=burnout, 2=malnourished, 3=stress>70, 4=loan>0, 5=rep>60
}

const DAILY_EVENTS: &[DailyEvt] = &[
    // ── Always available ──────────────────────────────────────────────────────
    DailyEvt {
        msg: "Payday bonus! +$50",
        eff: |s| {
            s.money += 50.;
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Unexpected bill! -$35",
        eff: |s| {
            s.money = (s.money - 35.).max(0.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Well rested! +15 Energy",
        eff: |s| {
            s.energy = (s.energy + 15.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Noisy night. -15 Energy",
        eff: |s| {
            s.energy = (s.energy - 15.).max(0.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Found $20!",
        eff: |s| {
            s.money += 20.;
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Beautiful morning! +15 Mood",
        eff: |s| {
            s.happiness = (s.happiness + 15.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Feeling ill. -10 Energy -10 Mood",
        eff: |s| {
            s.energy = (s.energy - 10.).max(0.);
            s.happiness = (s.happiness - 10.).max(0.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Kind message! +12 Mood",
        eff: |s| {
            s.happiness = (s.happiness + 12.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Subscription fee. -$20",
        eff: |s| {
            s.money = (s.money - 20.).max(0.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Doctor visit. +15 Health",
        eff: |s| {
            s.health = (s.health + 15.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Bad meal. -10 Health",
        eff: |s| {
            s.health = (s.health - 10.).max(0.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Tax refund! +$30",
        eff: |s| {
            s.money += 30.;
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Work relief. -15 Stress",
        eff: |s| {
            s.stress = (s.stress - 15.).max(0.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Overtime bonus! +$40",
        eff: |s| {
            s.money += 40.;
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Gym promo — free week! +20 Energy",
        eff: |s| {
            s.energy = (s.energy + 20.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Late delivery refund. +$10",
        eff: |s| {
            s.money += 10.;
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Community event. +10 Mood",
        eff: |s| {
            s.happiness = (s.happiness + 10.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Inspired by a book. +8 Mood",
        eff: |s| {
            s.happiness = (s.happiness + 8.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Argued with a neighbor. -18 Mood",
        eff: |s| {
            s.happiness = (s.happiness - 18.).max(0.);
        },
        rep: -2.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Power outage. -$15, +10 Stress",
        eff: |s| {
            s.money = (s.money - 15.).max(0.);
            s.stress = (s.stress + 10.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Late bill notice. +12 Stress",
        eff: |s| {
            s.stress = (s.stress + 12.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Internet outage — work delayed. -$20",
        eff: |s| {
            s.money = (s.money - 20.).max(0.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    DailyEvt {
        msg: "Parking fine. -$25",
        eff: |s| {
            s.money = (s.money - 25.).max(0.);
        },
        rep: 0.,
        season: 0,
        cond: 0,
    },
    // ── Spring ────────────────────────────────────────────────────────────────
    DailyEvt {
        msg: "Spring allergies. -12 Energy",
        eff: |s| {
            s.energy = (s.energy - 12.).max(0.);
        },
        rep: 0.,
        season: 1,
        cond: 0,
    },
    DailyEvt {
        msg: "Blooming city! +20 Mood",
        eff: |s| {
            s.happiness = (s.happiness + 20.).min(100.);
        },
        rep: 0.,
        season: 1,
        cond: 0,
    },
    DailyEvt {
        msg: "Spring cleaning gig! +$35",
        eff: |s| {
            s.money += 35.;
        },
        rep: 2.,
        season: 1,
        cond: 0,
    },
    DailyEvt {
        msg: "Outdoor festival! +18 Mood",
        eff: |s| {
            s.happiness = (s.happiness + 18.).min(100.);
        },
        rep: 0.,
        season: 1,
        cond: 0,
    },
    DailyEvt {
        msg: "April shower — refreshing! +8 Energy",
        eff: |s| {
            s.energy = (s.energy + 8.).min(100.);
        },
        rep: 0.,
        season: 1,
        cond: 0,
    },
    // ── Summer ────────────────────────────────────────────────────────────────
    DailyEvt {
        msg: "Heatwave! -18 Energy",
        eff: |s| {
            s.energy = (s.energy - 18.).max(0.);
        },
        rep: 0.,
        season: 2,
        cond: 0,
    },
    DailyEvt {
        msg: "Beach trip! +22 Mood",
        eff: |s| {
            s.happiness = (s.happiness + 22.).min(100.);
        },
        rep: 0.,
        season: 2,
        cond: 0,
    },
    DailyEvt {
        msg: "Summer work bonus! +$60",
        eff: |s| {
            s.money += 60.;
        },
        rep: 0.,
        season: 2,
        cond: 0,
    },
    DailyEvt {
        msg: "Sunstroke warning. -12 Health",
        eff: |s| {
            s.health = (s.health - 12.).max(0.);
        },
        rep: 0.,
        season: 2,
        cond: 0,
    },
    DailyEvt {
        msg: "Ice cream social! +10 Mood +5 Energy",
        eff: |s| {
            s.happiness = (s.happiness + 10.).min(100.);
            s.energy = (s.energy + 5.).min(100.);
        },
        rep: 0.,
        season: 2,
        cond: 0,
    },
    // ── Autumn ────────────────────────────────────────────────────────────────
    DailyEvt {
        msg: "Harvest festival! +15 Mood",
        eff: |s| {
            s.happiness = (s.happiness + 15.).min(100.);
        },
        rep: 0.,
        season: 3,
        cond: 0,
    },
    DailyEvt {
        msg: "Autumn flu. -15 Health",
        eff: |s| {
            s.health = (s.health - 15.).max(0.);
        },
        rep: 0.,
        season: 3,
        cond: 0,
    },
    DailyEvt {
        msg: "Hobby fair income! +$20",
        eff: |s| {
            s.money += 20.;
        },
        rep: 0.,
        season: 3,
        cond: 0,
    },
    DailyEvt {
        msg: "Crisp air jog. +12 Energy +5 Mood",
        eff: |s| {
            s.energy = (s.energy + 12.).min(100.);
            s.happiness = (s.happiness + 5.).min(100.);
        },
        rep: 0.,
        season: 3,
        cond: 0,
    },
    DailyEvt {
        msg: "Back-to-school sale. -$18",
        eff: |s| {
            s.money = (s.money - 18.).max(0.);
        },
        rep: 0.,
        season: 3,
        cond: 0,
    },
    // ── Winter ────────────────────────────────────────────────────────────────
    DailyEvt {
        msg: "Heating bill! -$30",
        eff: |s| {
            s.money = (s.money - 30.).max(0.);
        },
        rep: 0.,
        season: 4,
        cond: 0,
    },
    DailyEvt {
        msg: "Holiday bonus! +$70",
        eff: |s| {
            s.money += 70.;
        },
        rep: 0.,
        season: 4,
        cond: 0,
    },
    DailyEvt {
        msg: "Cold snap. -15 Energy",
        eff: |s| {
            s.energy = (s.energy - 15.).max(0.);
        },
        rep: 0.,
        season: 4,
        cond: 0,
    },
    DailyEvt {
        msg: "Hot soup recovery. +10 Health +8 Mood",
        eff: |s| {
            s.health = (s.health + 10.).min(100.);
            s.happiness = (s.happiness + 8.).min(100.);
        },
        rep: 0.,
        season: 4,
        cond: 0,
    },
    DailyEvt {
        msg: "New Year energy. -Stress +8 Mood",
        eff: |s| {
            s.stress = (s.stress - 10.).max(0.);
            s.happiness = (s.happiness + 8.).min(100.);
        },
        rep: 0.,
        season: 4,
        cond: 0,
    },
    // ── Conditional ───────────────────────────────────────────────────────────
    DailyEvt {
        msg: "Support group session. -20 Stress",
        eff: |s| {
            s.stress = (s.stress - 20.).max(0.);
        },
        rep: 0.,
        season: 0,
        cond: 1,
    },
    DailyEvt {
        msg: "Free nutrition clinic. +12 Health",
        eff: |s| {
            s.health = (s.health + 12.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 2,
    },
    DailyEvt {
        msg: "Stress headache. -10 Energy -8 Health",
        eff: |s| {
            s.energy = (s.energy - 10.).max(0.);
            s.health = (s.health - 8.).max(0.);
        },
        rep: 0.,
        season: 0,
        cond: 3,
    },
    DailyEvt {
        msg: "Debt reminder letter. +15 Stress",
        eff: |s| {
            s.stress = (s.stress + 15.).min(100.);
        },
        rep: 0.,
        season: 0,
        cond: 4,
    },
    DailyEvt {
        msg: "Reputation perk: premium gig! +$45",
        eff: |s| {
            s.money += 45.;
        },
        rep: 5.,
        season: 0,
        cond: 5,
    },
    DailyEvt {
        msg: "VIP invite! +15 Mood",
        eff: |s| {
            s.happiness = (s.happiness + 15.).min(100.);
        },
        rep: 3.,
        season: 0,
        cond: 5,
    },
];

pub fn tick_time(mut gt: ResMut<GameTime>, time: Res<Time>, mut sfx: EventWriter<PlaySfx>) {
    let dt = time.delta_secs();
    gt.anim_secs += dt;
    gt.hours += dt * TIME_SCALE / 3600.;
    if gt.hours >= 24. {
        gt.hours -= 24.;
        gt.day += 1;
        sfx.send(PlaySfx(SfxKind::Chime));
    }
}

// ── on_new_day helpers ────────────────────────────────────────────────────────

fn tick_conditions(
    stats: &PlayerStats,
    gs: &GameState,
    conds: &mut Conditions,
    notif: &mut Notification,
) {
    if gs.high_stress_today {
        conds.burnout_days += 1;
        if conds.burnout_days >= 3 && !conds.burnout {
            conds.burnout = true;
            notif.push("Burnout! Work pay -30%. Rest up.", 6.);
        }
    } else if stats.stress < 40. {
        if conds.burnout_days > 0 {
            conds.burnout_days -= 1;
        }
        if conds.burnout_days == 0 && conds.burnout {
            conds.burnout = false;
            notif.push(
                "Recovery complete! Burnout cleared. Work pay back to 100%.",
                5.,
            );
        }
    }
    if gs.high_hunger_today {
        conds.malnourish_days += 1;
        if conds.malnourish_days >= 3 && !conds.malnourished {
            conds.malnourished = true;
            notif.push("Malnourished! Health drains faster.", 6.);
        }
    } else if stats.hunger < 40. {
        if conds.malnourish_days > 0 {
            conds.malnourish_days -= 1;
        }
        if conds.malnourish_days == 0 && conds.malnourished {
            conds.malnourished = false;
            notif.push("Eating well! Malnourishment cleared. Health stable.", 5.);
        }
    }
    if stats.stress > 75. {
        conds.high_stress_days += 1;
        conds.low_stress_days = 0;
        if conds.high_stress_days >= 3 && !conds.mental_fatigue {
            conds.mental_fatigue = true;
            notif.push("Mental Fatigue! Chronic stress is taking its toll. Work pay -15%. Relax or meditate.", 7.);
        }
    } else if stats.stress < 45. {
        conds.high_stress_days = 0;
        if conds.mental_fatigue {
            conds.low_stress_days += 1;
            if conds.low_stress_days >= 2 {
                conds.mental_fatigue = false;
                conds.low_stress_days = 0;
                notif.push(
                    "Mental Fatigue cleared! Stress is under control. Work pay restored.",
                    5.,
                );
            }
        }
    }
}

fn tick_investments(gt: &GameTime, invest: &mut Investment, notif: &mut Notification) {
    if invest.amount <= 0. || invest.risk == 0 {
        return;
    }
    let seed = gt
        .day
        .wrapping_mul(1664525)
        .wrapping_add(invest.risk as u32 * 999983);
    let mood_seed = gt.day.wrapping_mul(2246822519).wrapping_add(997);
    let pos_chance: u32 = if mood_seed % 7 < 2 {
        7
    } else if mood_seed % 7 >= 5 {
        4
    } else {
        5
    };
    let rand_sign: f32 = if seed % 10 < pos_chance { 1.0 } else { -1.0 };
    let rand_frac: f32 = (seed % 1000) as f32 / 1000.0;
    let daily_rate = invest.daily_return_rate * rand_sign * (0.5 + rand_frac * 0.5);
    let change = invest.amount * daily_rate;
    invest.amount = (invest.amount + change).max(0.);
    invest.total_return += change;
    if change.abs() >= 1. {
        let dir = if change > 0. { "gained" } else { "lost" };
        let mood_label = if pos_chance >= 7 {
            " [Bull]"
        } else if pos_chance <= 4 {
            " [Bear]"
        } else {
            ""
        };
        notif.push(
            format!(
                "Investment{} {} ${:.0}! Portfolio: ${:.0}",
                mood_label,
                dir,
                change.abs(),
                invest.amount
            ),
            4.,
        );
    }
}

fn apply_rent(
    stats: &mut PlayerStats,
    housing: &mut HousingTier,
    gs: &mut GameState,
    notif: &mut Notification,
    rent_mult: f32,
    crisis_rent_mult: f32,
) {
    let rent = housing.rent() * rent_mult * crisis_rent_mult;
    if stats.money >= rent {
        stats.money -= rent;
        stats.unpaid_rent_days = 0;
    } else {
        stats.unpaid_rent_days += 1;
        stats.happiness = (stats.happiness - 20.).max(0.);
        stats.stress = (stats.stress + 20.).min(100.);
        if stats.unpaid_rent_days >= 3 {
            stats.money = 0.;
            stats.health = (stats.health - 20.).max(0.);
            stats.unpaid_rent_days = 0;
            *housing = HousingTier::Unhoused;
            gs.just_evicted = true;
            notif.push("Evicted! Your lease is gone. Find a new place to stay.", 8.);
        } else {
            notif.push(
                format!(
                    "Cannot pay rent (${:.0})! {} day(s) unpaid.",
                    rent, stats.unpaid_rent_days
                ),
                6.,
            );
        }
    }
}

fn decay_friendships(friendship: &mut NpcFriendship, skills: &Skills) {
    let social_master = skills.social >= 5.0;
    let not_chatted: Vec<Entity> = friendship
        .levels
        .keys()
        .filter(|e| !friendship.chatted_today.get(*e).copied().unwrap_or(false))
        .cloned()
        .collect();
    for e in not_chatted {
        if let Some(lvl) = friendship.levels.get_mut(&e) {
            let base_decay = if *lvl >= 4. {
                0.05
            } else if *lvl >= 2. {
                0.10
            } else {
                0.15
            };
            let decay = if social_master {
                base_decay * 0.5
            } else {
                base_decay
            };
            *lvl = (*lvl - decay).max(0.);
        }
    }
    friendship.chatted_today.clear();
}

fn decay_skills(gs: &GameState, skills: &mut Skills) {
    const SKILL_FLOOR: f32 = 0.5;
    if gs.work_today == 0 && gs.study_today == 0 && skills.career > SKILL_FLOOR {
        skills.career = (skills.career - 0.01).max(SKILL_FLOOR);
    }
    if gs.exercise_today == 0 && skills.fitness > SKILL_FLOOR {
        skills.fitness = (skills.fitness - 0.01).max(SKILL_FLOOR);
    }
    if gs.chat_today == 0 && skills.social > SKILL_FLOOR {
        skills.social = (skills.social - 0.01).max(SKILL_FLOOR);
    }
    if gs.eat_today == 0 && skills.cooking > SKILL_FLOOR {
        skills.cooking = (skills.cooking - 0.01).max(SKILL_FLOOR);
    }
}

fn reset_daily_state(
    gt: &GameTime,
    gs: &mut GameState,
    goal: &mut DailyGoal,
    friendship: &mut NpcFriendship,
    social_events: &mut SocialEvents,
    quest_board: &mut QuestBoard,
    season: &Season,
    housing: &HousingTier,
    stats: &PlayerStats,
) {
    social_events.party_today = false;
    quest_board.crafted_today = 0;
    friendship.gifted_today.clear();
    gs.days_survived = gt.day;
    gs.money_earned_today = 0.;
    gs.work_today = 0;
    gs.eat_today = 0;
    gs.chat_today = 0;
    gs.exercise_today = 0;
    gs.hobby_today = 0;
    gs.passive_income_today = 0.;
    gs.outdoor_done_today = false;
    gs.study_today = 0;
    gs.high_stress_today = false;
    gs.high_hunger_today = false;
    *goal = make_goal(gt.day, &season.current);

    if *housing == HousingTier::Unhoused && stats.savings < 90. {
        *goal = DailyGoal {
            kind: GoalKind::SaveMoney,
            description: "Save $90 at the bank - get an apartment".to_string(),
            progress: 0.,
            target: 90.,
            reward_money: 20.,
            reward_happiness: 25.,
            completed: false,
            failed: false,
        };
    }
}

fn apply_daily_event(
    gt: &GameTime,
    stats: &mut PlayerStats,
    conds: &Conditions,
    rep: &mut Reputation,
    notif: &mut Notification,
    season: &Season,
) {
    let cur_season = match season.current {
        SeasonKind::Spring => 1u8,
        SeasonKind::Summer => 2,
        SeasonKind::Autumn => 3,
        SeasonKind::Winter => 4,
    };
    let pool: Vec<usize> = (0..DAILY_EVENTS.len())
        .filter(|&i| {
            let e = &DAILY_EVENTS[i];
            if e.season != 0 && e.season != cur_season {
                return false;
            }
            if gt.day <= 1 && e.msg.contains("-$") {
                return false;
            }
            match e.cond {
                1 => conds.burnout,
                2 => conds.malnourished,
                3 => stats.stress > 70.,
                4 => stats.loan > 0.,
                5 => rep.score > 60.,
                _ => true,
            }
        })
        .collect();

    let seed = (gt.day as usize).wrapping_mul(2654435761);
    let pick1 = pool[seed % pool.len()];
    let evt_idx = if rep.score > 60. {
        let pick2 = pool[seed.wrapping_mul(2246822519) % pool.len()];
        if DAILY_EVENTS[pick2].rep >= DAILY_EVENTS[pick1].rep {
            pick2
        } else {
            pick1
        }
    } else {
        pick1
    };

    let evt = &DAILY_EVENTS[evt_idx];
    (evt.eff)(stats);
    rep.score = (rep.score + evt.rep).clamp(0., 100.);
    notif.push(format!("Day {} - {}", gt.day + 1, evt.msg), 5.);
}

pub fn on_new_day(
    mut gt: ResMut<GameTime>,
    mut stats: ResMut<PlayerStats>,
    mut gs: ResMut<GameState>,
    mut goal: ResMut<DailyGoal>,
    mut notif: ResMut<Notification>,
    mut rating: ResMut<LifeRating>,
    mut streak: ResMut<WorkStreak>,
    mut friendship: ResMut<NpcFriendship>,
    mut housing: ResMut<HousingTier>,
    mut skills: ResMut<Skills>,
    mut weather: ResMut<WeatherKind>,
    hobbies: Res<Hobbies>,
    mut conds: ResMut<Conditions>,
    mut invest: ResMut<Investment>,
    mut rep: ResMut<Reputation>,
    mut day_extras: DayExtras,
) {
    if gt.day == gt.prev_day {
        return;
    }
    gt.prev_day = gt.day;

    rating.sample(&stats, &skills);

    // ── Season update ─────────────────────────────────────────────────────────
    day_extras.season.current = SeasonKind::from_day(gt.day);

    // ── Weather update ────────────────────────────────────────────────────────
    *weather = WeatherKind::from_day(gt.day);
    if weather.is_stormy() {
        stats.happiness = (stats.happiness - 8.).max(0.);
        notif.push(
            format!("Day {} — Stormy! -8 Mood. Stay indoors.", gt.day + 1),
            4.,
        );
    }

    // ── Condition resolution ──────────────────────────────────────────────────
    tick_conditions(&stats, &gs, &mut conds, &mut notif);

    // ── Pet hunger daily tick ─────────────────────────────────────────────────
    if day_extras.pet.has_pet {
        if !day_extras.pet.fed_today {
            day_extras.pet.hunger = (day_extras.pet.hunger + 30.).min(100.);
            stats.happiness = (stats.happiness - 10.).max(0.);
            notif.push(
                format!("{} is hungry! Feed your pet.", day_extras.pet.name),
                4.,
            );
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
        notif.push(
            format!("Passive income: +${:.0} from hobbies!", passive),
            3.,
        );
    }

    // ── Skill mastery passive bonuses ─────────────────────────────────────────
    if skills.career >= 5.0 {
        stats.money += 10.;
        gs.passive_income_today += 10.;
        notif.push("Executive network: +$10 passive income!", 3.);
    }

    // ── Investment daily return ───────────────────────────────────────────────
    tick_investments(&gt, &mut invest, &mut notif);

    // ── Reputation decay ──────────────────────────────────────────────────────
    rep.score = (rep.score - 1.0).max(0.);

    // ── Loan daily interest 8% ────────────────────────────────────────────────
    if stats.loan > 0. {
        stats.loan *= 1. + 0.08 * day_extras.settings.difficulty.loan_interest_mult();
        stats.stress = (stats.stress + 5.).min(100.);
        if stats.loan > 300. {
            notif.push(
                format!("Loan is now ${:.0}! Pay it off at the Bank.", stats.loan),
                5.,
            );
        }
    }

    // ── Savings 5% interest ───────────────────────────────────────────────────
    if stats.savings > 0. {
        stats.savings *= 1. + 0.05 * day_extras.settings.difficulty.economy_mult();
    }

    // ── Sleep debt accumulates 8h per day ────────────────────────────────────
    stats.sleep_debt = (stats.sleep_debt + 8.).min(32.);

    // ── Season indoor/outdoor bonus at day start ──────────────────────────────
    let winter_bonus = day_extras.season.current.indoor_bonus();
    if winter_bonus > 0. {
        stats.happiness = (stats.happiness + winter_bonus).min(100.);
    }

    // ── Housing morning bonus ─────────────────────────────────────────────────
    let hap_bonus = housing.morning_hap();
    if hap_bonus > 0. {
        stats.happiness = (stats.happiness + hap_bonus).min(100.);
    }

    // ── Rent ──────────────────────────────────────────────────────────────────
    apply_rent(
        &mut stats,
        &mut housing,
        &mut gs,
        &mut notif,
        day_extras.settings.difficulty.rent_mult(),
        day_extras.crisis.rent_multiplier(),
    );

    // ── Work streak reset ─────────────────────────────────────────────────────
    if !streak.worked_today && streak.days > 0 {
        streak.days = 0;
        notif.push("Work streak broken!", 3.);
    }
    streak.worked_today = false;

    // ── Friendship decay ──────────────────────────────────────────────────────
    decay_friendships(&mut friendship, &skills);

    // ── Coffee expiry (perishable) ────────────────────────────────────────────
    if day_extras.inv.coffee > 0 {
        day_extras.inv.coffee_age += 1;
        if day_extras.inv.coffee_age >= 5 {
            day_extras.inv.coffee -= 1;
            day_extras.inv.coffee_age = 0;
            notif.push("A coffee went stale and was tossed!", 3.);
        }
    } else {
        day_extras.inv.coffee_age = 0;
    }

    // ── Skill decay — use it or lose it (0.01/day, floor 0.5) ───────────────
    decay_skills(&gs, &mut skills);

    // ── Reset daily state ─────────────────────────────────────────────────────
    reset_daily_state(
        &gt,
        &mut gs,
        &mut goal,
        &mut friendship,
        &mut day_extras.social_events,
        &mut day_extras.quest_board,
        &day_extras.season,
        &housing,
        &stats,
    );

    // Auto-save at the start of each new day.
    day_extras.save_writer.send_default();

    if gt.is_weekend() {
        notif.push(format!("It's {}! Work pays 1.5x today.", gt.day_name()), 4.);
    }

    // ── Daily random event — pool filtered by season, conditions, rep ─────────
    apply_daily_event(
        &gt,
        &mut stats,
        &conds,
        &mut rep,
        &mut notif,
        &day_extras.season,
    );
}

/// Teleports the player to the park zone center when evicted so they aren't
/// stuck inside a home they no longer own.
pub fn apply_eviction_teleport(
    mut gs: ResMut<GameState>,
    mut player_q: Query<&mut Transform, With<Player>>,
) {
    if !gs.just_evicted {
        return;
    }
    gs.just_evicted = false;
    if let Ok(mut tf) = player_q.get_single_mut() {
        tf.translation.x = 85.;
        tf.translation.y = 180.;
    }
}

/// Best-friend passive perks applied once per day.
pub fn best_friend_perks(
    gt: Res<GameTime>,
    mut gs: ResMut<GameState>,
    mut stats: ResMut<PlayerStats>,
    mut rep: ResMut<Reputation>,
    mut notif: ResMut<Notification>,
    friendship: Res<NpcFriendship>,
    npc_q: Query<(Entity, &Npc)>,
) {
    // Only run on the new-day tick (same guard as on_new_day)
    if gt.day == 0 || gt.day == gs.bf_perk_day {
        return;
    }
    gs.bf_perk_day = gt.day;

    let mut perks: Vec<&str> = Vec::new();
    for (entity, npc) in &npc_q {
        let lvl = friendship.levels.get(&entity).copied().unwrap_or(0.);
        if lvl < 5. {
            continue;
        }
        match npc.personality {
            NpcPersonality::Cheerful => {
                stats.happiness = (stats.happiness + 5.).min(100.);
                perks.push("Cheerful BF: +5 Happiness");
            }
            NpcPersonality::Wise => {
                stats.health = (stats.health + 3.).min(100.);
                perks.push("Wise BF: +3 Health");
            }
            NpcPersonality::Influential => {
                rep.score = (rep.score + 3.).clamp(0., 100.);
                perks.push("Influential BF: +3 Rep");
            }
            NpcPersonality::Neutral => {
                stats.money += 5.;
                perks.push("Best Friend: +$5");
            }
        }
    }
    if !perks.is_empty() {
        notif.push(format!("Best Friend perks: {}", perks.join(", ")), 4.);
    }
}

fn goal(
    kind: GoalKind,
    description: &str,
    target: f32,
    reward_money: f32,
    reward_happiness: f32,
) -> DailyGoal {
    DailyGoal {
        kind,
        description: description.to_string(),
        progress: 0.,
        target,
        reward_money,
        reward_happiness,
        completed: false,
        failed: false,
    }
}

pub fn make_goal(day: u32, season: &SeasonKind) -> DailyGoal {
    let (seasonal_desc, seasonal_target) = season.seasonal_goal_desc();
    match day % 18 {
        0 => goal(
            GoalKind::SaveMoney,
            "Save $90 at the bank - get an apartment",
            90.,
            20.,
            25.,
        ),
        1 => goal(GoalKind::WorkTimes, "Work 3 times today", 3., 30., 0.),
        2 => goal(
            GoalKind::MaintainHappy,
            "Stay happy (60%+) all day",
            1.,
            15.,
            10.,
        ),
        3 => goal(GoalKind::EatTimes, "Eat 2 meals today", 2., 0., 20.),
        4 => goal(GoalKind::ChatTimes, "Chat with 2 people", 2., 10., 15.),
        5 => goal(
            GoalKind::FriendNpc,
            "Friendship 2 with any NPC",
            2.,
            20.,
            10.,
        ),
        6 => goal(GoalKind::SaveMoney, "Have $30+ in savings", 30., 15., 0.),
        7 => goal(
            GoalKind::ExerciseTimes,
            "Exercise 2 times today",
            2.,
            0.,
            20.,
        ),
        8 => goal(GoalKind::LowerStress, "Get stress below 30", 30., 20., 10.),
        9 => goal(
            GoalKind::BuildStreak,
            "Maintain a 3-day work streak",
            3.,
            35.,
            5.,
        ),
        10 => goal(
            GoalKind::MasterHobby,
            "Reach level 3 in any hobby",
            3.,
            30.,
            0.,
        ),
        11 => goal(
            GoalKind::EarnPassive,
            "Earn $10 passive income today",
            10.,
            25.,
            0.,
        ),
        12 => goal(
            GoalKind::OutdoorWeather,
            "Go outside on a Sunny day",
            1.,
            20.,
            15.,
        ),
        13 => goal(
            GoalKind::StudyTimes,
            "Study 2 times today ($30 ea)",
            2.,
            20.,
            5.,
        ),
        14 => goal(GoalKind::FeedPet, "Feed your pet today", 1., 15., 20.),
        15 => goal(
            GoalKind::ThrowParty,
            "Throw a party today ($40)",
            1.,
            10.,
            30.,
        ),
        16 => goal(GoalKind::OwnVehicle, "Own a Bike or Car", 1., 30., 0.),
        _ => goal(
            GoalKind::SeasonalGoal,
            seasonal_desc,
            seasonal_target,
            25.,
            10.,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::SeasonKind;

    // ── GameTime helpers ───────────────────────────────────────────────────────

    fn gt(hours: f32, day: u32) -> GameTime {
        GameTime {
            hours,
            day,
            prev_day: day,
            anim_secs: 0.,
        }
    }

    #[test]
    fn display_midnight() {
        let s = gt(0., 0).display();
        assert!(s.contains("12:00 AM"), "got: {s}");
    }

    #[test]
    fn display_noon() {
        let s = gt(12., 0).display();
        assert!(s.contains("12:00 PM"), "got: {s}");
    }

    #[test]
    fn display_afternoon() {
        let s = gt(14.5, 2).display();
        assert!(s.contains("02:30 PM"), "got: {s}");
    }

    #[test]
    fn display_includes_day_number() {
        let s = gt(8., 4).display();
        assert!(s.contains("Day 5"), "got: {s}");
    }

    #[test]
    fn is_weekend_correct() {
        assert!(!gt(8., 0).is_weekend()); // Mon
        assert!(!gt(8., 4).is_weekend()); // Fri
        assert!(gt(8., 5).is_weekend()); // Sat
        assert!(gt(8., 6).is_weekend()); // Sun
        assert!(!gt(8., 7).is_weekend()); // next Mon
    }

    #[test]
    fn is_night_correct() {
        assert!(gt(21., 0).is_night());
        assert!(gt(23.9, 0).is_night());
        assert!(gt(0., 0).is_night());
        assert!(gt(5.9, 0).is_night());
        assert!(!gt(6., 0).is_night());
        assert!(!gt(12., 0).is_night());
        assert!(!gt(20.9, 0).is_night());
    }

    #[test]
    fn work_time_tag_early_bird() {
        let g = gt(7., 0);
        let (mult, tag) = g.work_time_tag();
        assert!(mult > 1.0);
        assert!(tag.contains("Early Bird"));
    }

    #[test]
    fn work_time_tag_late_night() {
        let g = gt(22., 0);
        let (mult, tag) = g.work_time_tag();
        assert!(mult < 1.0);
        assert!(tag.contains("Late Night"));
    }

    #[test]
    fn work_time_tag_normal() {
        let g = gt(14., 0);
        let (mult, tag) = g.work_time_tag();
        assert!((mult - 1.0).abs() < f32::EPSILON);
        assert!(tag.is_empty());
    }

    #[test]
    fn is_breakfast_correct() {
        assert!(gt(6., 0).is_breakfast());
        assert!(gt(8.5, 0).is_breakfast());
        assert!(!gt(9., 0).is_breakfast());
        assert!(!gt(5.9, 0).is_breakfast());
    }

    #[test]
    fn exercise_mult_morning_bonus() {
        assert!((gt(6., 0).exercise_mult() - 1.25).abs() < 0.001);
        assert!((gt(12., 0).exercise_mult() - 1.0).abs() < f32::EPSILON);
    }

    // ── make_goal branches ────────────────────────────────────────────────────

    #[test]
    fn make_goal_day_0_save_money() {
        let g = make_goal(0, &SeasonKind::Spring);
        assert!(matches!(g.kind, GoalKind::SaveMoney));
        assert!((g.target - 90.).abs() < 0.001);
        assert!((g.reward_money - 20.).abs() < 0.001);
        assert!((g.reward_happiness - 25.).abs() < 0.001);
    }

    #[test]
    fn make_goal_day_1_work_times() {
        let g = make_goal(1, &SeasonKind::Summer);
        assert!(matches!(g.kind, GoalKind::WorkTimes));
        assert!((g.target - 3.).abs() < 0.001);
    }

    #[test]
    fn make_goal_day_7_exercise() {
        let g = make_goal(7, &SeasonKind::Autumn);
        assert!(matches!(g.kind, GoalKind::ExerciseTimes));
        assert!((g.target - 2.).abs() < 0.001);
    }

    #[test]
    fn make_goal_day_17_seasonal() {
        let g = make_goal(17, &SeasonKind::Winter);
        assert!(matches!(g.kind, GoalKind::SeasonalGoal));
        assert!(g.target > 0.);
    }

    #[test]
    fn make_goal_wraps_at_18() {
        // day % 18 == 0 for both day 0 and day 18
        let g0 = make_goal(0, &SeasonKind::Spring);
        let g18 = make_goal(18, &SeasonKind::Spring);
        assert!(matches!(g0.kind, GoalKind::SaveMoney));
        assert!(matches!(g18.kind, GoalKind::SaveMoney));
        assert!((g0.target - g18.target).abs() < 0.001);
    }

    #[test]
    fn all_goal_branches_have_positive_target() {
        for day in 0..18u32 {
            let g = make_goal(day, &SeasonKind::Autumn);
            assert!(g.target > 0., "day {day} has zero target");
        }
    }

    #[test]
    fn all_goal_branches_start_not_completed() {
        for day in 0..18u32 {
            let g = make_goal(day, &SeasonKind::Winter);
            assert!(!g.completed, "day {day} goal should start incomplete");
            assert!(!g.failed, "day {day} goal should not start failed");
        }
    }

    #[test]
    fn all_goal_branches_have_nonempty_description() {
        for day in 0..18u32 {
            let g = make_goal(day, &SeasonKind::Spring);
            assert!(!g.description.is_empty(), "day {day} has empty description");
        }
    }
}
