use bevy::prelude::*;
use crate::components::*;
use crate::resources::*;

pub fn handle_interaction(
    keys: Res<ButtonInput<KeyCode>>, nearby: Res<NearbyInteractable>,
    inter_q: Query<&Interactable>, npc_q: Query<&Npc>,
    gt: Res<GameTime>,
    mut stats: ResMut<PlayerStats>, mut inv: ResMut<Inventory>, mut skills: ResMut<Skills>,
    mut friendship: ResMut<NpcFriendship>, mut gs: ResMut<GameState>,
    mut notif: ResMut<Notification>, mut goal: ResMut<DailyGoal>,
    mut streak: ResMut<WorkStreak>, mut housing: ResMut<HousingTier>,
    mut extras: InteractExtras,
) {
    let pe = keys.just_pressed(KeyCode::KeyE);
    let pg = keys.just_pressed(KeyCode::KeyG);
    let p1 = keys.just_pressed(KeyCode::Digit1);
    let p2 = keys.just_pressed(KeyCode::Digit2);
    let p3 = keys.just_pressed(KeyCode::Digit3);
    let p4 = keys.just_pressed(KeyCode::Digit4);
    let p5 = keys.just_pressed(KeyCode::Digit5);
    let p6 = keys.just_pressed(KeyCode::Digit6);
    let p7 = keys.just_pressed(KeyCode::Digit7);
    let p8 = keys.just_pressed(KeyCode::Digit8);

    if (!pe && !pg && !p1 && !p2 && !p3 && !p4 && !p5 && !p6 && !p7 && !p8) || stats.cooldown > 0. { return; }
    let Some(entity) = nearby.entity else { return };
    let Ok(inter) = inter_q.get(entity) else { return };

    // ── Bank key shortcuts ────────────────────────────────────────────────────
    if matches!(&inter.action, ActionKind::Bank) {
        if p1 {
            if stats.money >= 20. { stats.money -= 20.; stats.savings += 20.; stats.stress = (stats.stress - 3.).max(0.); notif.message = format!("Deposited $20. Savings: ${:.0}", stats.savings); }
            else { notif.message = "Not enough cash!".into(); }
            if matches!(&goal.kind, GoalKind::SaveMoney) && !goal.completed { goal.progress = stats.savings; if goal.progress >= goal.target { goal.completed = true; stats.money += goal.reward_money; notif.message = format!("Savings goal done! +${:.0}", goal.reward_money); } }
            notif.timer = 2.5; stats.cooldown = 0.5; return;
        }
        if p2 {
            if stats.savings >= 20. { stats.savings -= 20.; stats.money += 20.; notif.message = format!("Withdrew $20. Savings: ${:.0}", stats.savings); }
            else { notif.message = "Not enough savings!".into(); }
            notif.timer = 2.; stats.cooldown = 0.5; return;
        }
        if p3 {
            let half = (stats.money / 2.).floor().max(0.);
            if half >= 1. { stats.money -= half; stats.savings += half; notif.message = format!("Deposited ${:.0}. Savings: ${:.0}", half, stats.savings); }
            else { notif.message = "Nothing to deposit.".into(); }
            notif.timer = 2.; stats.cooldown = 0.5; return;
        }
        if p4 {
            if stats.loan < 200. { stats.loan += 100.; stats.money += 100.; stats.stress = (stats.stress + 10.).min(100.); notif.message = format!("Took $100 loan (8%/day). Total: ${:.0}", stats.loan); }
            else { notif.message = "Loan limit reached! Repay first.".into(); }
            notif.timer = 3.; stats.cooldown = 0.5; return;
        }
        if p5 {
            let pay = stats.loan.min(50.).min(stats.money);
            if pay > 0. { stats.loan -= pay; stats.money -= pay; stats.stress = (stats.stress - 5.).max(0.); notif.message = format!("Repaid ${:.0}. Remaining: ${:.0}", pay, stats.loan); }
            else { notif.message = "Nothing to repay or no cash.".into(); }
            notif.timer = 2.5; stats.cooldown = 0.5; return;
        }
        if p6 {
            if stats.money >= 50. { stats.money -= 50.; extras.invest.amount += 50.; if extras.invest.risk == 0 { extras.invest.risk = 1; extras.invest.daily_return_rate = 0.04; } notif.message = format!("Invested $50 (Low risk). Portfolio: ${:.0}", extras.invest.amount); }
            else { notif.message = "Need $50 to invest!".into(); }
            notif.timer = 3.; stats.cooldown = 0.5; return;
        }
        if p7 {
            if stats.money >= 50. { stats.money -= 50.; extras.invest.amount += 50.; extras.invest.risk = 2; extras.invest.daily_return_rate = 0.10; notif.message = format!("Invested $50 (Medium risk). Portfolio: ${:.0}", extras.invest.amount); }
            else { notif.message = "Need $50!".into(); }
            notif.timer = 3.; stats.cooldown = 0.5; return;
        }
        if p8 {
            if extras.invest.amount > 0. { let amt = extras.invest.amount; stats.money += amt; extras.invest.amount = 0.; extras.invest.risk = 0; extras.invest.daily_return_rate = 0.; notif.message = format!("Withdrew entire investment: +${:.0}", amt); }
            else { notif.message = "No investment to withdraw.".into(); }
            notif.timer = 3.; stats.cooldown = 0.5; return;
        }
    }

    // ── Transport key shortcuts ───────────────────────────────────────────────
    if matches!(&inter.action, ActionKind::BuyTransport) {
        if p1 {
            if extras.transport.kind == TransportKind::Bike || extras.transport.kind == TransportKind::Car { notif.message = "Already have a vehicle!".into(); }
            else if stats.savings >= 80. { stats.savings -= 80.; extras.transport.kind = TransportKind::Bike; notif.message = "Bought a Bicycle! Work pays 1.1x bonus.".into(); }
            else { notif.message = "Need $80 in savings for a Bike!".into(); }
            notif.timer = 3.; stats.cooldown = 0.5; return;
        }
        if p2 {
            if extras.transport.kind == TransportKind::Car { notif.message = "Already have a Car!".into(); }
            else if stats.savings >= 300. { stats.savings -= 300.; extras.transport.kind = TransportKind::Car; notif.message = "Bought a Car! Work pays 1.2x bonus.".into(); }
            else { notif.message = "Need $300 in savings for a Car!".into(); }
            notif.timer = 3.; stats.cooldown = 0.5; return;
        }
    }

    if !pe && !pg { return; }

    match inter.action.clone() {
        ActionKind::Sleep => {
            let gain = housing.sleep_energy(gt.is_night());
            let health_gain = housing.night_health();
            stats.energy = (stats.energy + gain).min(stats.max_energy());
            stats.health = (stats.health + health_gain + 3.).min(100.);
            stats.stress = (stats.stress - 15.).max(0.);
            stats.sleep_debt = (stats.sleep_debt - 8.).max(0.);
            stats.cooldown = 3.;
            let tag = if gt.is_night() { "Night sleep" } else { "Daytime nap" };
            notif.message = format!("{} — +{:.0} Energy, -SleepDebt, -Stress", tag, gain);
            notif.timer = 2.;
        }
        ActionKind::Eat => {
            let reduction = 40. * skills.cooking_bonus();
            let breakfast_bonus = if gt.is_breakfast() { 10. } else { 0. };
            if stats.meals > 0 { stats.meals -= 1; }
            else if stats.money >= 10. { stats.money -= 10.; }
            else { notif.message = "No food or money!".into(); notif.timer = 2.5; return; }
            stats.hunger = (stats.hunger - reduction).max(0.);
            stats.health = (stats.health + 1.).min(100.);
            stats.energy = (stats.energy + breakfast_bonus).min(stats.max_energy());
            skills.cooking = (skills.cooking + 0.10).min(5.);
            gs.eat_today += 1; stats.cooldown = 2.;
            let bfast = if breakfast_bonus > 0. { " [Breakfast +10 Energy!]" } else { "" };
            notif.message = format!("Ate! -{:.0} Hunger{}", reduction, bfast);
            notif.timer = 2.;
        }
        ActionKind::Work => {
            if stats.energy < 15. { notif.message = "Too tired to work!".into(); notif.timer = 2.; return; }
            if stats.health < 20. { notif.message = "Too sick to work!".into(); notif.timer = 2.; return; }
            if stats.stress > 90. { notif.message = "Too stressed! Meditate or relax first.".into(); notif.timer = 3.; return; }
            let mood = Mood::from_happiness(stats.happiness);
            let (time_mult, time_tag) = gt.work_time_tag();
            let weekend = if gt.is_weekend() { 1.5 } else { 1.0 };
            let burnout_mult = if extras.conds.burnout { 0.70 } else { 1.0 };
            let transport_mult = extras.transport.kind.work_bonus();
            let earned = skills.work_pay(streak.days) * mood.work_mult() * time_mult * weekend
                       * stats.stress_work_mult() * stats.loan_penalty() * burnout_mult * transport_mult;
            stats.money += earned;
            stats.energy = (stats.energy - 15.).max(0.);
            stats.happiness = (stats.happiness - 5.).max(0.);
            stats.stress = (stats.stress + 8.).min(100.);
            skills.career = (skills.career + 0.15).min(5.);
            gs.work_today += 1; gs.money_earned_today += earned;
            streak.worked_today = true; streak.days += 1;
            extras.rep.score = (extras.rep.score + 0.5).min(100.);
            stats.cooldown = 2.;
            let we = if gt.is_weekend() { " [Weekend 1.5x]" } else { "" };
            let st = if streak.days >= 3 { format!(" [Streak {}d]", streak.days) } else { String::new() };
            let bt = if extras.conds.burnout { " [Burnout -30%]" } else { "" };
            let tr = match extras.transport.kind { TransportKind::Bike => " [Bike +10%]", TransportKind::Car => " [Car +20%]", _ => "" };
            notif.message = format!("[{}]{}{}{}{}{} Earned ${:.0}!", skills.career_rank(), we, time_tag, st, bt, tr, earned);
            notif.timer = 2.5;
        }
        ActionKind::Freelance => {
            if stats.energy < 8. { notif.message = "Too tired for freelance!".into(); notif.timer = 2.; return; }
            let mood = Mood::from_happiness(stats.happiness);
            let base_pay = if skills.career >= 5.0 { 35. } else if skills.career >= 2.5 { 22. } else { 15. };
            let earned = base_pay * mood.work_mult() * stats.loan_penalty();
            stats.money += earned;
            stats.energy = (stats.energy - 8.).max(0.);
            stats.stress = (stats.stress + 3.).min(100.);
            gs.work_today += 1; gs.money_earned_today += earned;
            streak.worked_today = true;
            extras.rep.score = (extras.rep.score + 0.3).min(100.);
            stats.cooldown = 2.;
            notif.message = format!("Freelanced from home. Earned ${:.0}.", earned);
            notif.timer = 2.;
        }
        ActionKind::Shop => {
            if stats.money >= 15. {
                stats.money -= 15.; stats.meals += 3;
                if stats.money >= 5.  { stats.money -= 5.;  inv.coffee   += 1; }
                if stats.money >= 8.  { stats.money -= 8.;  inv.vitamins += 1; }
                if stats.money >= 12. { stats.money -= 12.; inv.books    += 1; }
                stats.cooldown = 1.;
                notif.message = "Shopped: +3 meals, +items if affordable.".into();
                notif.timer = 2.5;
            } else { notif.message = "Not enough money!".into(); notif.timer = 2.; }
        }
        ActionKind::Relax => {
            if extras.weather.is_stormy() { notif.message = "Stormy outside! Can't relax here.".into(); notif.timer = 2.; return; }
            let gain = 20. * skills.social_bonus();
            let weather_bonus = extras.weather.outdoor_hap_bonus();
            let season_bonus  = extras.season.current.outdoor_bonus();
            stats.happiness = (stats.happiness + gain + weather_bonus + season_bonus).min(100.);
            stats.energy = (stats.energy - 3.).max(0.);
            stats.stress = (stats.stress - 12.).max(0.);
            skills.fitness = (skills.fitness + 0.08).min(5.);
            gs.outdoor_done_today = true;
            stats.cooldown = 3.;
            let wb = if weather_bonus + season_bonus > 0. { format!(" [+{:.0} outdoor bonus]", weather_bonus + season_bonus) } else { String::new() };
            notif.message = format!("Relaxed. +{:.0} Happiness, -Stress.{}", gain, wb);
            notif.timer = 2.;
        }
        ActionKind::Exercise => {
            if stats.energy < 20. { notif.message = "Too tired to exercise!".into(); notif.timer = 2.; return; }
            if extras.weather.is_stormy() { notif.message = "Stormy! Exercise indoors instead.".into(); notif.timer = 2.; return; }
            let season_mult = extras.season.current.social_mult();
            let fit_gain = (0.20 + skills.fitness * 0.02) * gt.exercise_mult() * season_mult;
            stats.energy = (stats.energy - 20.).max(0.);
            stats.health = (stats.health + 8.).min(100.);
            stats.hunger = (stats.hunger + 10.).min(100.);
            stats.happiness = (stats.happiness + 10.).min(100.);
            stats.stress = (stats.stress - 8.).max(0.);
            skills.fitness = (skills.fitness + fit_gain).min(5.);
            gs.exercise_today += 1; gs.outdoor_done_today = true;
            stats.cooldown = 3.;
            let am = if gt.exercise_mult() > 1.0 { " [Morning +25%]" } else { "" };
            let sp = if season_mult > 1.0 { " [Spring +25%]" } else { "" };
            notif.message = format!("Exercised! +Health, +Fitness {:.2}{}{}", fit_gain, am, sp);
            notif.timer = 2.5;
        }
        ActionKind::Meditate => {
            let hap_gain = 15. + skills.social * 2.;
            stats.happiness = (stats.happiness + hap_gain).min(100.);
            stats.stress = (stats.stress - 25.).max(0.);
            stats.meditation_buff = 300.;
            stats.cooldown = 4.;
            notif.message = format!("Meditated. +{:.0} Happiness, -Stress, Zen buff 5h.", hap_gain);
            notif.timer = 3.;
        }
        ActionKind::Shower => {
            stats.happiness = (stats.happiness + 12.).min(100.);
            stats.health    = (stats.health + 2.).min(100.);
            stats.stress    = (stats.stress - 5.).max(0.);
            stats.cooldown  = 2.;
            notif.message = "Showered! +12 Happiness, +Health, -Stress.".into();
            notif.timer = 2.;
        }
        ActionKind::Bank => {
            if let Some(cost) = housing.upgrade_cost() {
                if stats.savings >= cost {
                    if let Some(next) = housing.next() {
                        stats.savings -= cost;
                        let label = next.label().to_string();
                        *housing = next;
                        notif.message = format!("Upgraded to {}! Savings -${:.0}", label, cost);
                        notif.timer = 5.;
                    }
                } else {
                    let next_label = housing.next().map(|h| h.label().to_string()).unwrap_or_else(|| "N/A".into());
                    notif.message = format!("Bank: ${:.0} saved. [1]Dep$20 [2]Wdraw [3]HalfDep [4]Loan+$100 [5]Repay$50 [6]Invest(low) [7]Invest(med) [8]CashOut. Upgrade: ${:.0} for {}.", stats.savings, cost, next_label);
                    notif.timer = 7.;
                }
            } else {
                notif.message = format!("Bank: ${:.0} saved. Max housing! [1-8]", stats.savings);
                notif.timer = 4.;
            }
            stats.cooldown = 0.5;
        }
        ActionKind::UseItem(kind) => {
            match kind {
                ItemKind::Coffee   => if inv.coffee   > 0 { inv.coffee   -= 1; stats.energy = (stats.energy+30.).min(stats.max_energy()); stats.cooldown=1.; notif.message=format!("Coffee! +30 Energy. ({}x left)", inv.coffee); }
                                      else { notif.message="No coffee!".into(); },
                ItemKind::Vitamins => if inv.vitamins > 0 { inv.vitamins -= 1; stats.health = (stats.health+15.).min(100.); stats.cooldown=1.; notif.message=format!("Vitamins! +15 Health. ({}x left)", inv.vitamins); }
                                      else { notif.message="No vitamins!".into(); },
                ItemKind::Books    => if inv.books    > 0 { inv.books    -= 1; skills.career=(skills.career+0.5).min(5.); stats.cooldown=2.; notif.message=format!("Read! +0.5 Career XP. ({}x left)", inv.books); }
                                      else { notif.message="No books!".into(); },
            }
            notif.timer = 2.;
        }
        ActionKind::Hobby(kind) => {
            if stats.energy < 10. { notif.message = "Too tired for a hobby!".into(); notif.timer = 2.; return; }
            let (skill_val, label) = match kind {
                HobbyKind::Painting => (&mut extras.hobbies.painting, "Painting"),
                HobbyKind::Gaming   => (&mut extras.hobbies.gaming,   "Gaming"),
                HobbyKind::Music    => (&mut extras.hobbies.music,    "Music"),
            };
            *skill_val = (*skill_val + 0.25).min(5.);
            let lvl = *skill_val;
            let winter_bonus = extras.season.current.indoor_bonus();
            stats.happiness = (stats.happiness + 12. + winter_bonus).min(100.);
            stats.stress = (stats.stress - 8.).max(0.);
            stats.energy = (stats.energy - 10.).max(0.);
            gs.hobby_today += 1;
            stats.cooldown = 2.;
            let wb = if winter_bonus > 0. { " [Winter +cozy]" } else { "" };
            notif.message = format!("{}! Skill: {:.2}/5. +Happiness, -Stress.{}", label, lvl, wb);
            notif.timer = 2.5;
        }
        ActionKind::StudyCourse => {
            if stats.money < 30. { notif.message = "Need $30 to study!".into(); notif.timer = 2.; return; }
            if stats.energy < 20. { notif.message = "Too tired to study!".into(); notif.timer = 2.; return; }
            stats.money -= 30.; stats.energy -= 20.;
            let season_bonus = if extras.season.current == SeasonKind::Spring { 0.25 } else { 0. };
            let seed = (gt.day.wrapping_mul(1664525)).wrapping_add(gs.study_today as u32 * 999983) % 4;
            let (boost_name, new_lvl) = match seed {
                0 => { skills.cooking = (skills.cooking + 0.5 + season_bonus).min(5.); ("Cooking", skills.cooking) }
                1 => { skills.career  = (skills.career  + 0.5 + season_bonus).min(5.); ("Career",  skills.career)  }
                2 => { skills.fitness = (skills.fitness + 0.5 + season_bonus).min(5.); ("Fitness", skills.fitness) }
                _ => { skills.social  = (skills.social  + 0.5 + season_bonus).min(5.); ("Social",  skills.social)  }
            };
            gs.study_today += 1;
            stats.cooldown = 3.;
            let sp = if season_bonus > 0. { " [Spring +0.25 bonus!]" } else { "" };
            notif.message = format!("Studied! +{:.2} {} (now {:.1}){}", 0.5 + season_bonus, boost_name, new_lvl, sp);
            notif.timer = 3.;
        }
        ActionKind::FeedPet => {
            if !extras.pet.has_pet {
                if stats.money >= 50. {
                    stats.money -= 50.;
                    extras.pet.has_pet = true;
                    extras.pet.fed_today = true;
                    extras.pet.hunger = 0.;
                    notif.message = format!("Adopted {}! Feed daily for +happiness.", extras.pet.name);
                } else { notif.message = "Need $50 to adopt a pet!".into(); }
                notif.timer = 3.; stats.cooldown = 0.5; return;
            }
            if stats.money < 5. { notif.message = "Need $5 to feed your pet!".into(); notif.timer = 2.; return; }
            stats.money -= 5.;
            extras.pet.hunger = 0.;
            extras.pet.fed_today = true;
            stats.happiness = (stats.happiness + 15.).min(100.);
            stats.stress = (stats.stress - 5.).max(0.);
            stats.cooldown = 1.;
            notif.message = format!("Fed {}! +15 Happiness, -Stress.", extras.pet.name);
            notif.timer = 2.;
        }
        ActionKind::ThrowParty => {
            if stats.money < 40. { notif.message = "Need $40 to throw a party!".into(); notif.timer = 2.; return; }
            if stats.energy < 20. { notif.message = "Too tired to party!".into(); notif.timer = 2.; return; }
            stats.money -= 40.; stats.energy -= 20.;
            stats.happiness = (stats.happiness + 30.).min(100.);
            stats.stress = (stats.stress - 10.).max(0.);
            extras.social_events.parties_thrown += 1;
            extras.social_events.party_today = true;
            extras.rep.score = (extras.rep.score + 5.).min(100.);
            stats.cooldown = 4.;
            notif.message = format!("Party thrown! +30 Happiness, +5 Rep. ({} total)", extras.social_events.parties_thrown);
            notif.timer = 4.;
        }
        ActionKind::BuyTransport => {
            notif.message = format!("Transport: {}. [1] Bike $80sav [2] Car $300sav. Work bonuses: Bike +10%, Car +20%.", extras.transport.kind.label());
            notif.timer = 5.; stats.cooldown = 0.5;
        }
        ActionKind::Chat => {
            if pg {
                let lvl = friendship.levels.get(&entity).copied().unwrap_or(0.);
                if lvl >= 5. && stats.money >= 10. {
                    stats.money -= 10.; stats.happiness = (stats.happiness + 25.).min(100.); stats.cooldown = 2.;
                    let name = npc_q.get(entity).map(|n| n.name.as_str()).unwrap_or("them");
                    notif.message = format!("Gift to {}! +25 Happiness.", name);
                } else if lvl < 5. { notif.message = "Need max friendship (5) to gift!".into(); }
                  else { notif.message = "Need $10 for a gift.".into(); }
                notif.timer = 2.; return;
            }
            if !pe { return; }
            let f = friendship.levels.entry(entity).or_insert(0.);
            let chat_bonus = extras.rep.chat_bonus() * extras.season.current.social_mult();
            let gain_mult = (1. + (*f * 0.05)) * chat_bonus;
            let hap = 15. * skills.social_bonus() * gain_mult;
            *f = (*f + 0.30).min(5.);
            let lvl = *f;
            friendship.chatted_today.insert(entity, true);
            stats.happiness = (stats.happiness + hap).min(100.);
            stats.stress = (stats.stress - 5.).max(0.);
            skills.social = (skills.social + 0.15).min(5.);
            gs.chat_today += 1; stats.cooldown = 1.5;
            extras.rep.score = (extras.rep.score + 0.8).min(100.);
            let name = npc_q.get(entity).map(|n| n.name.as_str()).unwrap_or("them");
            let sp = if extras.season.current == SeasonKind::Spring { " [Spring social!]" } else { "" };
            notif.message = format!("Chat with {}! +{:.0} Mood  Friendship {}/5{}", name, hap, lvl as u32, sp);
            notif.timer = 2.5;
        }
    }
}