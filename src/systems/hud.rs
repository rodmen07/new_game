use bevy::prelude::*;
use crate::components::*;
use crate::resources::*;

pub fn tick_notification(mut notif: ResMut<Notification>, time: Res<Time>) {
    if notif.timer > 0. {
        notif.timer = (notif.timer - time.delta_secs()).max(0.);
        if notif.timer == 0. { notif.message.clear(); }
    }
}

pub fn update_hud(
    stats: Res<PlayerStats>, gt: Res<GameTime>, nearby: Res<NearbyInteractable>,
    skills: Res<Skills>, notif: Res<Notification>, goal: Res<DailyGoal>,
    friendship: Res<NpcFriendship>, inv: Res<Inventory>, rating: Res<LifeRating>,
    ms: Res<Milestones>, housing: Res<HousingTier>, streak: Res<WorkStreak>,
    extras: HudExtras,
    npc_q: Query<(Entity, &Npc)>,
    mut labels: Query<(&HudLabel, &mut Text)>,
    mut bars:   Query<(&HudBar, &mut Node, &mut BackgroundColor)>,
) {
    let mood = Mood::from_happiness(stats.happiness);
    let friend_str: String = {
        let mut p = Vec::new();
        for (e, n) in &npc_q {
            let lvl = *friendship.levels.get(&e).unwrap_or(&0.) as u32;
            p.push(format!("{} {}/5", n.name, lvl));
        }
        p.join("  ")
    };

    for (label, mut text) in &mut labels {
        *text = Text::new(match label {
            HudLabel::Time    => gt.display(),
            HudLabel::Money   => format!("${:.0} cash  savings ${:.0}  | {} meals", stats.money, stats.savings, stats.meals),
            HudLabel::Rent    => if stats.unpaid_rent_days == 0 { String::new() } else { format!("! {} unpaid rent day(s)!", stats.unpaid_rent_days) },
            HudLabel::Mood    => {
                let med = if stats.meditation_buff > 0. { " [Zen]" } else { "" };
                let debt = if stats.sleep_debt > 8. { format!(" [SleepDebt {:.0}h]", stats.sleep_debt) } else { String::new() };
                format!("Mood: {}{} | {}{}", mood.label(), med, skills.career_rank(), debt)
            }
            HudLabel::Prompt       => nearby.prompt.clone(),
            HudLabel::Warning      => warnings(&stats),
            HudLabel::Notification => notif.message.clone(),
            HudLabel::Skills       => format!("Cook {:.1}   Career {:.1}\nFit  {:.1}   Social {:.1}", skills.cooking, skills.career, skills.fitness, skills.social),
            HudLabel::Friendship   => friend_str.clone(),
            HudLabel::Inventory    => format!("Coffee x{}  Vitamins x{}  Books x{}", inv.coffee, inv.vitamins, inv.books),
            HudLabel::Streak  => {
                let streak_tag = if streak.days >= 7 { " BLAZING!" } else if streak.days >= 3 { " hot!" } else { "" };
                format!("Streak: {}d{}  Loan: ${:.0}", streak.days, streak_tag, stats.loan)
            }
            HudLabel::Housing => {
                let upgrade = housing.upgrade_cost().map(|c| format!("  Upgrade: ${:.0} savings", c)).unwrap_or_else(|| "  [MAX]".into());
                format!("{} | ${:.0}/day{}", housing.label(), housing.rent(), upgrade)
            }
            HudLabel::Rating  => format!("{}\nScore: {:.0}/100  ({} days)", rating.grade(), rating.score, rating.days),
            HudLabel::Milestones => format!("{} ({}/15)", ms.summary(), ms.count()),
            HudLabel::Goal => {
                let status = if goal.completed   { " [DONE!]".into() }
                             else if goal.failed { " [failed]".into() }
                             else { match &goal.kind {
                                GoalKind::MaintainHappy => if goal.failed { " [Off track]".into() } else { " [On track]".into() },
                                GoalKind::LowerStress   => format!(" (stress {:.0}/100)", stats.stress),
                                _ => format!(" ({:.0}/{:.0})", goal.progress, goal.target),
                             }};
                let rwd = match (goal.reward_money > 0., goal.reward_happiness > 0.) {
                    (true,true) => format!("+${:.0} +{:.0}Mood", goal.reward_money, goal.reward_happiness),
                    (true,_)   => format!("+${:.0}", goal.reward_money),
                    (_,true)   => format!("+{:.0}Mood", goal.reward_happiness),
                    _          => String::new(),
                };
                format!("{}{}\nReward: {}", goal.description, status, rwd)
            }
            HudLabel::Weather => {
                let (name, desc) = match *extras.weather {
                    WeatherKind::Sunny  => ("Sunny",  "outdoor bonus"),
                    WeatherKind::Cloudy => ("Cloudy", "mild day"),
                    WeatherKind::Rainy  => ("Rainy",  "stay active indoors"),
                    WeatherKind::Stormy => ("Stormy", "outdoor blocked"),
                };
                format!("{} — {}", name, desc)
            }
            HudLabel::Hobbies => {
                let h = &*extras.hobbies;
                format!("Paint {:.1}  Game {:.1}  Music {:.1}", h.painting, h.gaming, h.music)
            }
            HudLabel::Conditions => {
                let c = &*extras.conds;
                let mut parts: Vec<&str> = Vec::new();
                if c.burnout     { parts.push("Burnout"); }
                if c.malnourished{ parts.push("Malnourished"); }
                if parts.is_empty() { "Healthy".into() } else { parts.join(", ") }
            }
            HudLabel::Reputation => format!("Rep: {:.0}/100", extras.rep.score),
            HudLabel::Season => {
                let s = &extras.season.current;
                let bonus = match s {
                    SeasonKind::Spring => "social +25%",
                    SeasonKind::Summer => "outdoor +8hap",
                    SeasonKind::Autumn => "passive +25%",
                    SeasonKind::Winter => "indoor +5hap",
                };
                format!("{}  — {}", s.label(), bonus)
            }
            HudLabel::Pet => {
                if extras.pet.has_pet {
                    let fed = if extras.pet.fed_today { "Fed ✓" } else { "Hungry!" };
                    format!("{} — {}", extras.pet.name, fed)
                } else {
                    "No pet (adopt at Pet Bowl $50)".into()
                }
            }
            HudLabel::Transport => format!("{}", extras.transport.kind.label()),
        });
    }

    for (bar, mut node, mut bg) in &mut bars {
        let pct = match bar {
            HudBar::Energy    => stats.energy,
            HudBar::Hunger    => 100. - stats.hunger,
            HudBar::Happiness => stats.happiness,
            HudBar::Health    => stats.health,
            HudBar::Stress    => stats.stress,
        };
        node.width = Val::Percent(pct.clamp(0., 100.));
        *bg = BackgroundColor(bar_color(bar, pct));
    }
}

fn bar_color(bar: &HudBar, pct: f32) -> Color {
    match bar {
        HudBar::Stress => {
            if pct > 75.      { Color::srgb(0.9, 0.2, 0.15) }
            else if pct > 45. { Color::srgb(0.95, 0.55, 0.15) }
            else              { Color::srgb(0.3, 0.85, 0.35) }
        }
        _ => {
            if pct < 25.      { Color::srgb(0.9, 0.2, 0.15) }
            else if pct < 50. { Color::srgb(0.95, 0.55, 0.15) }
            else { match bar {
                HudBar::Energy    => Color::srgb(1.0, 0.78, 0.2),
                HudBar::Hunger    => Color::srgb(1.0, 0.55, 0.25),
                HudBar::Happiness => Color::srgb(0.4, 0.75, 1.0),
                HudBar::Health    => Color::srgb(0.3, 0.90, 0.4),
                HudBar::Stress    => unreachable!(),
            }}
        }
    }
}

fn warnings(s: &PlayerStats) -> String {
    let mut w = Vec::new();
    if s.energy < 20.      { w.push("! Exhausted"); }
    if s.hunger  > 75.     { w.push("! Starving"); }
    if s.happiness < 20.   { w.push("! Depressed"); }
    if s.money < 5.        { w.push("! Broke"); }
    if s.health < 30.      { w.push("! Sick"); }
    if s.stress > 75.      { w.push("! Stressed"); }
    if s.sleep_debt > 16.  { w.push("! Sleep-Deprived"); }
    if s.loan > 200.       { w.push("! Heavy Debt"); }
    if s.critical_timer > 10. { w.push("!! COLLAPSING"); }
    w.join("  ")
}