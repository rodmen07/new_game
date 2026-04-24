#![allow(clippy::type_complexity)]

use crate::components::*;
use crate::resources::*;
use bevy::prelude::*;
use bevy_tweening::{Animator, Lens, Targetable, Tween};
use std::time::Duration;

// ── Notification slide-in lens ────────────────────────────────────────────────

/// Tweens the `top` field of a `Node` so the notification panel slides in
/// from above the viewport instead of popping in instantly.
struct NotifTopLens {
    start: f32,
    end: f32,
}
impl Lens<Node> for NotifTopLens {
    fn lerp(&mut self, target: &mut dyn Targetable<Node>, ratio: f32) {
        let v = self.start + (self.end - self.start) * ratio;
        let node = std::ops::DerefMut::deref_mut(target);
        node.top = Val::Px(v);
    }
}

/// Detects when a new notification message starts and triggers a slide-in
/// tween on the `NotifContainer` node.
pub fn animate_notification(
    mut prev: Local<String>,
    notif: Res<Notification>,
    notif_q: Query<Entity, With<NotifContainer>>,
    mut commands: Commands,
) {
    if notif.message == *prev {
        return;
    }
    *prev = notif.message.clone();
    if notif.message.is_empty() {
        return;
    }
    let Some(entity) = notif_q.iter().next() else {
        return;
    };
    let tween = Tween::new(
        EaseFunction::QuadraticOut,
        Duration::from_millis(280),
        NotifTopLens {
            start: -60.,
            end: 12.,
        },
    );
    commands.entity(entity).insert(Animator::new(tween));
}

pub fn tick_notification(mut notif: ResMut<Notification>, time: Res<Time>) {
    notif.tick(time.delta_secs());
}

#[allow(clippy::too_many_arguments)]
pub fn update_hud(
    player_q: Query<
        (&PlayerStats, &Skills, &Inventory, &HousingTier, &WorkStreak),
        With<LocalPlayer>,
    >,
    gt: Res<GameTime>,
    nearby: Res<NearbyInteractable>,
    notif: Res<Notification>,
    goal: Res<DailyGoal>,
    friendship: Res<NpcFriendship>,
    rating: Res<LifeRating>,
    ms: Res<Milestones>,
    extras: HudExtras,
    npc_q: Query<(Entity, &Npc)>,
    mut labels: Query<(&HudLabel, &mut Text, &mut TextColor)>,
    mut bars: Query<(&HudBar, &mut BarSmooth)>,
) {
    let Some((stats, skills, inv, housing, streak)) = player_q.iter().next() else {
        return;
    };
    let mood = Mood::from_happiness(stats.happiness);
    let friend_str: String = {
        let entries: Vec<String> = npc_q
            .iter()
            .map(|(e, n)| {
                let lvl = *friendship.levels.get(&e).unwrap_or(&0.) as u32;
                format!("{} {}/5", n.name, lvl)
            })
            .collect();
        let mid = entries.len().div_ceil(2);
        let cap_line = |s: String| -> String {
            if s.len() > 52 {
                format!("{}…", &s[..51])
            } else {
                s
            }
        };
        if entries.len() > 3 {
            format!(
                "{}\n{}",
                cap_line(entries[..mid].join("  ")),
                cap_line(entries[mid..].join("  "))
            )
        } else {
            cap_line(entries.join("  "))
        }
    };
    let in_vehicle = extras
        .player_vehicle_q
        .iter()
        .next()
        .map(|vehicle_state| vehicle_state.in_vehicle)
        .unwrap_or(false);

    for (label, mut text, mut text_color) in &mut labels {
        *text = Text::new(match label {
            HudLabel::Time => {
                let work_hint = if gt.hours >= 6. && gt.hours < 10. {
                    "  [Early Bird +15%]"
                } else if gt.hours >= 20. {
                    "  [Late Night -10%]"
                } else if gt.is_weekend() {
                    "  [Weekend 1.5x]"
                } else {
                    ""
                };
                format!(
                    "{}  [{}]{}",
                    gt.display(),
                    extras.settings.difficulty.label(),
                    work_hint
                )
            }
            HudLabel::Money => {
                let meals = if stats.meals > 9999 {
                    "9999+".to_string()
                } else {
                    stats.meals.to_string()
                };
                let cash_label = if stats.money < 0. { "DEBT" } else { "cash" };
                let s = format!(
                    "{} {} | {} saved | {} meals",
                    fmt_cash(stats.money),
                    cash_label,
                    fmt_cash(stats.savings),
                    meals
                );
                if stats.money < 0. {
                    *text_color = TextColor(Color::srgb(1.0, 0.25, 0.2));
                } else {
                    *text_color = TextColor(Color::srgb(0.4, 1., 0.5));
                }
                if s.len() > 56 {
                    format!("{}…", &s[..55])
                } else {
                    s
                }
            }
            HudLabel::Rent => {
                if stats.unpaid_rent_days == 0 {
                    String::new()
                } else {
                    format!("! {} unpaid rent day(s)!", stats.unpaid_rent_days)
                }
            }
            HudLabel::Mood => {
                let med = if stats.meditation_buff > 0. {
                    " [Zen]"
                } else {
                    ""
                };
                let debt = if stats.sleep_debt > 8. {
                    format!(" [SleepDebt {:.0}h]", stats.sleep_debt)
                } else {
                    String::new()
                };
                format!(
                    "Mood: {}{} | {}{}",
                    mood.label(),
                    med,
                    skills.career_rank(),
                    debt
                )
            }
            HudLabel::Prompt => {
                if let Some(prompt) = extras.player_prompt_q.iter().next()
                    && prompt.active
                {
                    String::new() // typing overlay handles display
                } else if !nearby.prompt.is_empty() {
                    nearby.prompt.clone()
                } else if stats.hunger > 70. && stats.money >= 12. {
                    "Tip: Cafe (E) - quick hunger fix".to_string()
                } else if stats.energy < 25. && !housing.has_access() {
                    "Tip: Rest at the park shelter (N) to recover energy".to_string()
                } else if !housing.has_access() && stats.money >= 90. {
                    "Tip: Bank (SW) - deposit $90 to sign your lease".to_string()
                } else if gt.day == 0 && stats.money < 40. && !housing.has_access() {
                    "Tip: Office (NE) pays best - work to earn cash".to_string()
                } else {
                    String::new()
                }
            }
            HudLabel::Warning => warnings(stats, &extras.conds),
            HudLabel::Notification => notif.message.clone(),
            HudLabel::Skills => {
                let career_next = if skills.career >= 5.0 {
                    ""
                } else if skills.career >= 2.5 {
                    "  → Exec @ 5.0"
                } else {
                    "  → Senior @ 2.5"
                };
                format!(
                    "Cook {:.1}   Career {:.1}{}\nFit  {:.1}   Social {:.1}",
                    skills.cooking, skills.career, career_next, skills.fitness, skills.social
                )
            }
            HudLabel::Friendship => friend_str.clone(),
            HudLabel::Inventory => {
                let mut parts: Vec<String> = Vec::new();
                if inv.coffee > 0 {
                    let fresh = 5u32.saturating_sub(inv.coffee_age);
                    parts.push(format!("Coffee x{} ({}d fresh)", inv.coffee, fresh));
                }
                if inv.vitamins > 0 {
                    parts.push(format!("Vitamins x{}", inv.vitamins));
                }
                if inv.books > 0 {
                    parts.push(format!("Books x{}", inv.books));
                }
                let mut extras_vec: Vec<String> = Vec::new();
                if inv.ingredient > 0 {
                    extras_vec.push(format!("Ingredient x{}", inv.ingredient));
                }
                if inv.gift_box > 0 {
                    extras_vec.push(format!("GiftBox x{}", inv.gift_box));
                }
                if inv.smoothie > 0 {
                    extras_vec.push(format!("Smoothie x{}", inv.smoothie));
                }
                if parts.is_empty() && extras_vec.is_empty() {
                    "Empty bag — Shop (E) sells supplies".to_string()
                } else if extras_vec.is_empty() {
                    parts.join("  ")
                } else if parts.is_empty() {
                    extras_vec.join("  ")
                } else {
                    format!("{}\n{}", parts.join("  "), extras_vec.join("  "))
                }
            }
            HudLabel::Streak => {
                let streak_tag = if streak.days >= 7 {
                    " BLAZING!"
                } else if streak.days >= 3 {
                    " hot!"
                } else {
                    ""
                };
                let days_str = if streak.days > 999 {
                    "999+".to_string()
                } else {
                    streak.days.to_string()
                };
                match (streak.days > 0, stats.loan > 0.) {
                    (true, true) => format!(
                        "Streak: {}d{}  Loan: {}",
                        days_str,
                        streak_tag,
                        fmt_cash(stats.loan)
                    ),
                    (true, false) => format!("Streak: {}d{}", days_str, streak_tag),
                    (false, true) => format!("Loan: {}", fmt_cash(stats.loan)),
                    _ => String::new(),
                }
            }
            HudLabel::Housing => {
                if !housing.has_access() {
                    let cost = housing.upgrade_cost().unwrap_or(90.);
                    let toward = (stats.savings + stats.money).min(cost);
                    format!(
                        "No Home {} ${:.0}/{:.0} — deposit at Bank",
                        mini_bar(toward, cost),
                        toward,
                        cost
                    )
                } else {
                    let upgrade = housing
                        .upgrade_cost()
                        .map(|c| format!("  Upgrade: ${:.0} savings", c))
                        .unwrap_or_else(|| "  [MAX]".to_string());
                    format!(
                        "{} | ${:.0}/day{}",
                        housing.label(),
                        housing.rent(),
                        upgrade
                    )
                }
            }
            HudLabel::Rating => {
                let days_str = if rating.days > 9999 {
                    "9999+".to_string()
                } else {
                    rating.days.to_string()
                };
                format!(
                    "{}\nScore: {:.0}/100  ({} days)",
                    rating.grade(),
                    rating.score,
                    days_str
                )
            }
            HudLabel::Milestones => {
                let s = ms.summary();
                let display = if s.len() > 60 {
                    format!("{}...", &s[..57])
                } else {
                    s
                };
                format!("{} ({}/21)", display, ms.count())
            }
            HudLabel::Goal => {
                let status = if goal.completed {
                    " [DONE!]".to_string()
                } else if goal.failed {
                    " [failed]".to_string()
                } else {
                    match &goal.kind {
                        GoalKind::MaintainHappy => {
                            if goal.failed {
                                " [Off track]".to_string()
                            } else {
                                format!(" [On track - {}hap]", stats.happiness as u32)
                            }
                        }
                        GoalKind::LowerStress => {
                            let remaining = (100. - stats.stress).max(0.);
                            let needed = (100. - goal.target).max(0.001);
                            format!(
                                " {} stress {:.0} → {:.0}",
                                mini_bar(remaining, needed),
                                stats.stress,
                                goal.target
                            )
                        }
                        _ => format!(
                            " {} {}/{}",
                            mini_bar(goal.progress, goal.target),
                            goal.progress as u32,
                            goal.target as u32
                        ),
                    }
                };
                let rwd = match (goal.reward_money > 0., goal.reward_happiness > 0.) {
                    (true, true) => format!(
                        "+${:.0} +{:.0}Mood",
                        goal.reward_money, goal.reward_happiness
                    ),
                    (true, _) => format!("+${:.0}", goal.reward_money),
                    (_, true) => format!("+{:.0}Mood", goal.reward_happiness),
                    _ => String::new(),
                };
                format!("{}{}\nReward: {}", goal.description, status, rwd)
            }
            HudLabel::Story => {
                let summary = format!(
                    "Chapter {}\n{}",
                    extras.story.count().max(1),
                    extras.story.latest_summary()
                );
                if summary.len() > 104 {
                    format!("{}...", &summary[..101])
                } else {
                    summary
                }
            }
            HudLabel::Weather => {
                let is_winter = extras.season.current == SeasonKind::Winter;
                let (label, desc) = match *extras.weather {
                    WeatherKind::Sunny => ("Sunny", "outdoor bonus"),
                    WeatherKind::Cloudy => match extras.season.current {
                        SeasonKind::Autumn => ("Cloudy", "leaves drifting"),
                        SeasonKind::Spring => ("Cloudy", "petals floating"),
                        _ => ("Cloudy", "mild day"),
                    },
                    WeatherKind::Rainy if is_winter => ("Snowing", "stay warm indoors"),
                    WeatherKind::Rainy => ("Rainy", "stay active indoors"),
                    WeatherKind::Stormy if is_winter => ("Blizzard", "heavy snow, stay inside"),
                    WeatherKind::Stormy => ("Stormy", "thunder + outdoor blocked"),
                };
                format!("{} - {}", label, desc)
            }
            HudLabel::Hobbies => {
                let h = &*extras.hobbies;
                format!(
                    "Paint {:.1}  Game {:.1}  Music {:.1}",
                    h.painting, h.gaming, h.music
                )
            }
            HudLabel::Conditions => {
                let c = &*extras.conds;
                let mut parts: Vec<String> = Vec::new();
                if c.hospitalized {
                    parts.push("Hospitalised".to_string());
                }
                if c.burnout {
                    parts.push("Burnout (pay -30%)".to_string());
                } else if c.burnout_days >= 2 {
                    parts.push("Overworked (burnout risk)".to_string());
                }
                if c.malnourished {
                    parts.push("Malnourished (health-)".to_string());
                } else if c.malnourish_days >= 2 {
                    parts.push("Poor Diet (mal. risk)".to_string());
                }
                if c.mental_fatigue {
                    parts.push("Mental Fatigue (pay -15%)".to_string());
                } else if c.high_stress_days >= 2 {
                    parts.push("Chronic Stress (fatigue risk)".to_string());
                }
                if let Some(kind) = &extras.crisis.active
                    && extras.crisis.days_left > 0
                {
                    parts.push(format!("{} ({}d)", kind.label(), extras.crisis.days_left));
                }
                if extras.crisis.has_insurance {
                    parts.push(format!("Insured ({}d)", extras.crisis.insurance_days));
                }
                if parts.is_empty() {
                    "Healthy".to_string()
                } else {
                    parts.join("  ")
                }
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
                let fest = if let Some(fk) = &extras.festival.active {
                    format!(" | {} ({}tok)", fk.label(), extras.festival.tokens)
                } else {
                    String::new()
                };
                format!("{} - {}{}", s.label(), bonus, fest)
            }
            HudLabel::Pet => {
                if extras.pet.has_pet {
                    let fed = if extras.pet.fed_today {
                        "Fed ✓"
                    } else {
                        "Hungry!"
                    };
                    format!(
                        "{} ({}) - {}",
                        extras.pet.name,
                        extras.pet.kind.label(),
                        fed
                    )
                } else {
                    "No pet - visit Adoption Center".to_string()
                }
            }
            HudLabel::Transport => {
                let bonus_pct = ((extras.transport.kind.work_bonus() - 1.0) * 100.0).round() as i32;
                if in_vehicle {
                    format!(
                        "{} (driving, +{}% pay)",
                        extras.transport.kind.label(),
                        bonus_pct
                    )
                } else if extras.transport.kind.is_vehicle() {
                    if extras.transport.maintenance_due {
                        format!(
                            "{}  [Needs Service! Garage [3] $15 — pay bonus suspended]",
                            extras.transport.kind.label()
                        )
                    } else {
                        format!(
                            "{}  (+{}% pay, {}/5 uses until service)",
                            extras.transport.kind.label(),
                            bonus_pct,
                            extras.transport.work_uses
                        )
                    }
                } else {
                    format!(
                        "{}  Garage: Bike +10% / Car +20%",
                        extras.transport.kind.label()
                    )
                }
            }
            HudLabel::Quest => {
                let qb = &*extras.quest_board;
                if qb.quests.is_empty() {
                    format!(
                        "No quests - chat with NPCs [Q] (done: {})",
                        qb.completed_total
                    )
                } else {
                    let lines: Vec<String> = qb
                        .quests
                        .iter()
                        .filter(|q| !q.completed)
                        .take(3)
                        .map(|q| format!("{} {}/{}", q.description, q.progress, q.target))
                        .collect();
                    let summary = lines.join("  |  ");
                    if summary.len() > 80 {
                        format!("{}... (done: {})", &summary[..77], qb.completed_total)
                    } else {
                        format!("{}  (done: {})", summary, qb.completed_total)
                    }
                }
            }
        });
    }

    for (bar, mut smooth) in &mut bars {
        smooth.target = match bar {
            HudBar::Energy => stats.energy,
            HudBar::Hunger => 100. - stats.hunger,
            HudBar::Happiness => stats.happiness,
            HudBar::Health => stats.health,
            HudBar::Stress => stats.stress,
        }
        .clamp(0., 100.);
    }
}

/// Smoothly animates stat bars toward their targets each frame.
pub fn smooth_bars(
    time: Res<Time>,
    mut bars: Query<(&HudBar, &mut Node, &mut BackgroundColor, &mut BarSmooth)>,
) {
    for (bar, mut node, mut bg, mut smooth) in &mut bars {
        let step = time.delta_secs() * 300.;
        if (smooth.displayed - smooth.target).abs() <= step {
            smooth.displayed = smooth.target;
        } else if smooth.displayed < smooth.target {
            smooth.displayed += step;
        } else {
            smooth.displayed -= step;
        }
        node.width = Val::Percent(smooth.displayed);
        *bg = BackgroundColor(bar_color(bar, smooth.displayed));
    }
}

fn bar_color(bar: &HudBar, pct: f32) -> Color {
    match bar {
        HudBar::Stress => {
            if pct > 75. {
                Color::srgb(0.9, 0.2, 0.15)
            } else if pct > 45. {
                Color::srgb(0.95, 0.55, 0.15)
            } else {
                Color::srgb(0.3, 0.85, 0.35)
            }
        }
        _ => {
            if pct < 25. {
                Color::srgb(0.9, 0.2, 0.15)
            } else if pct < 50. {
                Color::srgb(0.95, 0.55, 0.15)
            } else {
                match bar {
                    HudBar::Energy => Color::srgb(1.0, 0.78, 0.2),
                    HudBar::Hunger => Color::srgb(1.0, 0.55, 0.25),
                    HudBar::Happiness => Color::srgb(0.4, 0.75, 1.0),
                    HudBar::Health => Color::srgb(0.3, 0.90, 0.4),
                    HudBar::Stress => unreachable!(),
                }
            }
        }
    }
}

fn mini_bar(val: f32, max: f32) -> String {
    let pct = (val / max.max(0.001)).clamp(0., 1.);
    let filled = (pct * 8.).round() as usize;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(8 - filled))
}

fn fmt_cash(v: f32) -> String {
    if v.abs() >= 1_000_000. {
        format!("${:.1}M", v / 1_000_000.)
    } else if v.abs() >= 10_000. {
        format!("${:.1}k", v / 1_000.)
    } else {
        format!("${:.0}", v)
    }
}

fn warnings(s: &PlayerStats, conds: &Conditions) -> String {
    let mut w: Vec<&str> = Vec::new();
    if conds.hospitalized {
        w.push("!! HOSPITALISED");
    }
    if s.critical_timer > 10. {
        w.push("!! COLLAPSING");
    }

    if s.energy < 15. {
        w.push("! Exhausted");
    } else if s.energy < 30. {
        w.push("Low Energy");
    }

    if s.hunger > 82. {
        w.push("! Starving");
    } else if s.hunger > 62. {
        w.push("Hungry");
    }

    if s.happiness < 20. {
        w.push("! Depressed");
    } else if s.happiness < 38. {
        w.push("Low Mood");
    }

    if s.health < 25. {
        w.push("! Sick");
    } else if s.health < 50. {
        w.push("Poor Health");
    }

    if s.stress > 82. {
        w.push("! Stressed");
    } else if s.stress > 62. {
        w.push("Tense");
    }

    if s.sleep_debt > 16. {
        w.push("! Sleep-Deprived");
    } else if s.sleep_debt > 8. {
        w.push("Tired");
    }

    if s.money < 5. {
        w.push("! Broke");
    }
    if s.loan > 200. {
        w.push("! Heavy Debt");
    }

    w.join("  ")
}

// ── Skill panel ───────────────────────────────────────────────────────────────

/// Toggles the skill panel on/off when the `ToggleSkillPanel` action fires (Tab key).
pub fn toggle_skill_panel(
    mut actions: EventReader<PlayerAction>,
    mut panel_q: Query<&mut Visibility, With<SkillPanel>>,
) {
    let mut toggle = false;
    for action in actions.read() {
        if matches!(action, PlayerAction::ToggleSkillPanel) {
            toggle = true;
        }
    }
    if !toggle {
        return;
    }
    for mut vis in &mut panel_q {
        *vis = if *vis == Visibility::Hidden {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Renders the skill values into the panel bars every frame.
///
/// Each bar shows filled dots (●) for whole levels and a half-dot (◑) for the
/// fractional part, up to a max of 5 pips.
pub fn update_skill_panel(
    player_q: Query<&Skills, With<LocalPlayer>>,
    mut cooking_q: Query<
        &mut Text,
        (
            With<SkillCookingBar>,
            Without<SkillCareerBar>,
            Without<SkillFitnessBar>,
            Without<SkillSocialBar>,
        ),
    >,
    mut career_q: Query<
        &mut Text,
        (
            With<SkillCareerBar>,
            Without<SkillCookingBar>,
            Without<SkillFitnessBar>,
            Without<SkillSocialBar>,
        ),
    >,
    mut fitness_q: Query<
        &mut Text,
        (
            With<SkillFitnessBar>,
            Without<SkillCookingBar>,
            Without<SkillCareerBar>,
            Without<SkillSocialBar>,
        ),
    >,
    mut social_q: Query<
        &mut Text,
        (
            With<SkillSocialBar>,
            Without<SkillCookingBar>,
            Without<SkillCareerBar>,
            Without<SkillFitnessBar>,
        ),
    >,
) {
    let Some(skills) = player_q.iter().next() else {
        return;
    };
    set_skill_bar(&mut cooking_q, skills.cooking);
    set_skill_bar(&mut career_q, skills.career);
    set_skill_bar(&mut fitness_q, skills.fitness);
    set_skill_bar(&mut social_q, skills.social);
}

fn set_skill_bar(q: &mut Query<&mut Text, impl bevy::ecs::query::QueryFilter>, value: f32) {
    let Some(mut t) = q.iter_mut().next() else {
        return;
    };
    let pips = 5usize;
    let filled = (value as usize).min(pips);
    let frac = value - value.floor();
    let mut bar = String::new();
    for i in 0..pips {
        if i < filled {
            bar.push('●');
        } else if i == filled && frac >= 0.5 {
            bar.push('◑');
        } else {
            bar.push('·');
        }
    }
    let rank = if value >= 5.0 {
        " Master"
    } else if value >= 2.5 {
        " Senior"
    } else {
        ""
    };
    **t = format!("{bar}{rank}");
}

/// Updates the full-screen typing overlay each frame to reflect the current
/// `ActionPrompt` state: shows/hides the overlay and highlights per-character.
#[allow(clippy::too_many_arguments)]
pub fn update_typing_overlay(
    time: Res<Time>,
    prompt_q: Query<&ActionPrompt, With<LocalPlayer>>,
    mut overlay_q: Query<
        (
            &mut Visibility,
            &mut BackgroundColor,
            &mut TypingOverlayFade,
        ),
        With<TypingOverlay>,
    >,
    mut label_q: Query<&mut Text, With<TypingLabel>>,
    mut typed_q: Query<&mut Text, (With<TypingWordTyped>, Without<TypingLabel>)>,
    mut cur_box_q: Query<&mut Visibility, (With<TypingWordCurrentBox>, Without<TypingOverlay>)>,
    mut cur_q: Query<
        &mut Text,
        (
            With<TypingWordCurrent>,
            Without<TypingLabel>,
            Without<TypingWordTyped>,
        ),
    >,
    mut remain_q: Query<
        &mut Text,
        (
            With<TypingWordRemaining>,
            Without<TypingLabel>,
            Without<TypingWordTyped>,
            Without<TypingWordCurrent>,
        ),
    >,
    mut instr_q: Query<
        &mut Text,
        (
            With<TypingInstruction>,
            Without<TypingLabel>,
            Without<TypingWordTyped>,
            Without<TypingWordCurrent>,
            Without<TypingWordRemaining>,
        ),
    >,
    mut retries_q: Query<
        &mut Text,
        (
            With<TypingRetries>,
            Without<TypingLabel>,
            Without<TypingWordTyped>,
            Without<TypingWordCurrent>,
            Without<TypingWordRemaining>,
            Without<TypingInstruction>,
        ),
    >,
) {
    let Some(prompt) = prompt_q.iter().next() else {
        return;
    };
    let Some((mut vis, mut bg, mut fade)) = overlay_q.iter_mut().next() else {
        return;
    };
    if !prompt.active {
        // Snap alpha to 0 and hide.
        fade.alpha = 0.;
        bg.0 = Color::srgba(0., 0., 0., 0.);
        *vis = Visibility::Hidden;
        if let Some(mut t) = label_q.iter_mut().next() {
            **t = String::new();
        }
        if let Some(mut t) = typed_q.iter_mut().next() {
            **t = String::new();
        }
        if let Some(mut t) = cur_q.iter_mut().next() {
            **t = String::new();
        }
        if let Some(mut t) = remain_q.iter_mut().next() {
            **t = String::new();
        }
        if let Some(mut t) = instr_q.iter_mut().next() {
            **t = String::new();
        }
        if let Some(mut t) = retries_q.iter_mut().next() {
            **t = String::new();
        }
        return;
    }
    // Fade in: lerp alpha toward TARGET_ALPHA at 10 units/sec.
    let dt = time.delta_secs();
    let target = TypingOverlayFade::TARGET_ALPHA;
    fade.alpha = (fade.alpha + dt * 10.).min(target);
    bg.0 = Color::srgba(0., 0., 0., fade.alpha);
    *vis = Visibility::Visible;

    let expected_chars: Vec<char> = prompt.expected.chars().collect();
    let typed_len = prompt.buffer.chars().count().min(expected_chars.len());
    let typed_str: String = expected_chars[..typed_len].iter().collect();
    let cur_str: String = expected_chars
        .get(typed_len)
        .map(|c| c.to_string())
        .unwrap_or_default();
    let remaining_str: String = if typed_len + 1 < expected_chars.len() {
        expected_chars[typed_len + 1..].iter().collect()
    } else {
        String::new()
    };

    if let Some(mut t) = label_q.iter_mut().next() {
        **t = prompt.label.to_uppercase();
    }
    if let Some(mut t) = typed_q.iter_mut().next() {
        **t = typed_str;
    }
    if let Some(mut v) = cur_box_q.iter_mut().next() {
        *v = if cur_str.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    if let Some(mut t) = cur_q.iter_mut().next() {
        **t = cur_str;
    }
    if let Some(mut t) = remain_q.iter_mut().next() {
        **t = remaining_str;
    }
    if let Some(mut t) = instr_q.iter_mut().next() {
        **t = prompt.instruction.clone();
    }
    if let Some(mut t) = retries_q.iter_mut().next() {
        **t = format!("{} retries left  [Esc] cancel", prompt.retries_left);
    }
}

/// Drives the entrance scale tween on the typing overlay's word row (P2-B).
///
/// While the prompt is active the row's scale lerps from
/// `TypingWordRowScale::START_SCALE` toward `TARGET_SCALE` at
/// `RATE_PER_SEC`, giving a roughly 120 ms ease. When the prompt is
/// inactive the scale snaps back so the next show repeats the entrance.
pub fn update_typing_word_row_scale(
    time: Res<Time>,
    prompt_q: Query<&ActionPrompt, With<LocalPlayer>>,
    mut row_q: Query<
        (&mut Transform, &mut TypingWordRowScale),
        (With<TypingWordRow>, Without<LocalPlayer>),
    >,
) {
    let active = prompt_q.iter().next().map(|p| p.active).unwrap_or(false);
    let dt = time.delta_secs();
    for (mut tf, mut row) in &mut row_q {
        row.scale = next_word_row_scale(row.scale, dt, active);
        tf.scale = Vec3::splat(row.scale);
    }
}

/// Pure helper: advances the word-row scale one frame.
/// When `active`, eases toward `TARGET_SCALE`; otherwise snaps to `START_SCALE`.
fn next_word_row_scale(current: f32, dt: f32, active: bool) -> f32 {
    if active {
        (current + dt * TypingWordRowScale::RATE_PER_SEC).min(TypingWordRowScale::TARGET_SCALE)
    } else {
        TypingWordRowScale::START_SCALE
    }
}

/// Drives the tutorial overlay: shows/hides it based on `TutorialState`,
/// advances on `Advance`/`Confirm` (Space/Enter), and dismisses on `Cancel` (Esc).
pub fn update_tutorial(
    mut actions: EventReader<PlayerAction>,
    mut state: ResMut<TutorialState>,
    mut overlay_q: Query<&mut Visibility, With<TutorialOverlay>>,
    mut body_q: Query<&mut Text, (With<TutorialBodyText>, Without<TutorialHintText>)>,
    mut hint_q: Query<&mut Text, (With<TutorialHintText>, Without<TutorialBodyText>)>,
) {
    // Advance or dismiss on action events.
    if state.is_active() {
        let mut cancelled = false;
        let mut advance = false;
        for a in actions.read() {
            match a {
                PlayerAction::Cancel => cancelled = true,
                PlayerAction::Advance | PlayerAction::Confirm => advance = true,
                _ => {}
            }
        }
        if cancelled {
            state.dismiss();
        } else if advance {
            state.advance();
        }
    }

    // Sync overlay visibility and text content.
    let visible = state.is_active();
    for mut vis in &mut overlay_q {
        *vis = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if visible && let Some((title, body)) = state.current() {
        let total = TUTORIAL_STEPS.len();
        if let Some(mut t) = hint_q.iter_mut().next() {
            **t = format!("{} / {}", state.step, total);
        }
        if let Some(mut t) = body_q.iter_mut().next() {
            **t = format!("{}\n\n{}", title, body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::next_word_row_scale;
    use crate::components::TypingWordRowScale;

    #[test]
    fn snaps_to_start_when_inactive() {
        let s = next_word_row_scale(0.92, 0.1, false);
        assert_eq!(s, TypingWordRowScale::START_SCALE);
    }

    #[test]
    fn eases_toward_target_when_active() {
        let s = next_word_row_scale(TypingWordRowScale::START_SCALE, 0.05, true);
        assert!(s > TypingWordRowScale::START_SCALE);
        assert!(s <= TypingWordRowScale::TARGET_SCALE);
    }

    #[test]
    fn clamps_to_target_when_active_long_dt() {
        let s = next_word_row_scale(TypingWordRowScale::START_SCALE, 10.0, true);
        assert_eq!(s, TypingWordRowScale::TARGET_SCALE);
    }

    #[test]
    fn already_at_target_stays_at_target() {
        let s = next_word_row_scale(TypingWordRowScale::TARGET_SCALE, 0.016, true);
        assert_eq!(s, TypingWordRowScale::TARGET_SCALE);
    }
}
