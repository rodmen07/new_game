#![allow(clippy::too_many_arguments)]

use crate::audio::{PlaySfx, SfxKind};
use crate::components::*;
use crate::constants::{OFFICE_BOTTOM, OFFICE_LEFT, OFFICE_RIGHT, OFFICE_TOP};
use crate::resources::*;
use crate::settings::GameSettings;
use bevy::prelude::*;

fn action_time_hours(action: &ActionKind) -> f32 {
    match action {
        ActionKind::Sleep => 8.0,
        ActionKind::Eat => 0.5,
        ActionKind::Work => 4.0,
        ActionKind::Freelance => 3.0,
        ActionKind::Shop => 0.5,
        ActionKind::Relax => 1.0,
        ActionKind::Shower => 0.25,
        ActionKind::Chat => 0.5,
        ActionKind::Exercise => 1.25,
        ActionKind::Meditate => 1.0,
        ActionKind::Bank => 0.25,
        ActionKind::UseItem(_) => 0.25,
        ActionKind::Hobby(_) => 1.5,
        ActionKind::StudyCourse => 2.0,
        ActionKind::FeedPet => 0.25,
        ActionKind::ThrowParty => 3.0,
        ActionKind::BuyTransport => 0.5,
        ActionKind::GymSession => 1.5,
        ActionKind::Cafe => 0.5,
        ActionKind::Clinic => 2.0,
        ActionKind::EnterVehicle => 0.,
        ActionKind::AdoptPet(_) => 0.25,
        ActionKind::SleepRough => 8.0,
        ActionKind::Craft => 0.5,
        ActionKind::RentUnit(_) => 0.5,
        ActionKind::GasUp => 0.25,
        ActionKind::RepairVehicle => 1.0,
        ActionKind::DentalVisit => 1.5,
        ActionKind::EyeExam => 1.0,
        ActionKind::ComputerLab => 2.0,
        ActionKind::PrintShop => 0.25,
    }
}

fn needs_home_access(action: &ActionKind) -> bool {
    matches!(
        action,
        ActionKind::Sleep
            | ActionKind::Freelance
            | ActionKind::Shower
            | ActionKind::Meditate
            | ActionKind::Hobby(_)
            | ActionKind::FeedPet
            | ActionKind::ThrowParty
            | ActionKind::UseItem(_)
            | ActionKind::Craft
    )
}

// ── Pure business-logic helpers (testable without ECS) ────────────────────────

fn health_work_mult(health: f32) -> f32 {
    if health < 25. {
        0.75
    } else if health < 50. {
        0.90
    } else {
        1.0
    }
}

fn freelance_base_pay(career: f32) -> f32 {
    if career >= 5.0 {
        35.
    } else if career >= 2.5 {
        22.
    } else {
        15.
    }
}

fn meal_tier(cooking: f32) -> (&'static str, f32, f32) {
    if cooking >= 5.0 {
        ("Master Chef meal", 10., 8.)
    } else if cooking >= 4.0 {
        ("Gourmet meal", 6., 5.)
    } else if cooking >= 2.0 {
        ("Good meal", 3., 0.)
    } else {
        ("Basic meal", 0., 0.)
    }
}

fn exercise_energy_cost(fitness: f32) -> f32 {
    if fitness >= 5.0 { 10. } else { 20. }
}

/// Returns `(new_money, new_savings)` on success, `None` if insufficient cash.
fn try_deposit(money: f32, savings: f32, amount: f32) -> Option<(f32, f32)> {
    (amount > 0. && money >= amount).then_some((money - amount, savings + amount))
}

/// Returns `(new_savings, new_money)` on success, `None` if insufficient savings.
fn try_withdraw(savings: f32, money: f32, amount: f32) -> Option<(f32, f32)> {
    (amount > 0. && savings >= amount).then_some((savings - amount, money + amount))
}

struct PromptChallenge {
    label: String,
    instruction: String,
    expected: String,
}

fn action_prompt_retries(career: f32) -> u8 {
    if career >= 5.0 {
        2
    } else if career >= 2.5 {
        3
    } else {
        4
    }
}

fn build_prompt_challenge(
    pending: &PendingAction,
    seed: u32,
    subject_name: &str,
) -> PromptChallenge {
    const OFFICE_WORDS: &[&str] = &[
        "desk",
        "report",
        "email",
        "meeting",
        "budget",
        "client",
        "memo",
        "office",
        "printer",
        "spreadsheet",
    ];
    const FOOD_WORDS: &[&str] = &["soup", "sandwich", "noodles", "salad", "stew", "toast"];
    const INGREDIENT_WORDS: &[&str] = &["carrot", "rice", "herb", "tomato", "pepper", "onion"];
    const DRINK_WORDS: &[&str] = &["latte", "tea", "mocha", "juice"];
    let pick = |pool: &'static [&'static str], offset: u32| -> &'static str {
        pool[(seed.wrapping_add(offset * 11) as usize) % pool.len()]
    };
    let subject = if subject_name.trim().is_empty() {
        "friend".to_string()
    } else {
        subject_name.trim().to_lowercase()
    };

    match pending {
        PendingAction::Action(ActionKind::Work) => PromptChallenge {
            label: "Work".to_string(),
            instruction: "type the office words in order".to_string(),
            expected: format!(
                "{} {} {}",
                pick(OFFICE_WORDS, 1),
                pick(OFFICE_WORDS, 2),
                pick(OFFICE_WORDS, 3)
            ),
        },
        PendingAction::Action(ActionKind::Freelance) => PromptChallenge {
            label: "Freelance".to_string(),
            instruction: "type the remote work words".to_string(),
            expected: format!(
                "{} {} {}",
                pick(OFFICE_WORDS, 4),
                pick(OFFICE_WORDS, 5),
                pick(OFFICE_WORDS, 6)
            ),
        },
        PendingAction::Action(ActionKind::Eat) => PromptChallenge {
            label: "Eat".to_string(),
            instruction: "type the meal command".to_string(),
            expected: format!("eat {}", pick(FOOD_WORDS, 1)),
        },
        PendingAction::Action(ActionKind::Sleep) => PromptChallenge {
            label: "Sleep".to_string(),
            instruction: "type the rest command".to_string(),
            expected: "sleep now".to_string(),
        },
        PendingAction::Action(ActionKind::SleepRough) => PromptChallenge {
            label: "Shelter".to_string(),
            instruction: "type the shelter command".to_string(),
            expected: "rest shelter".to_string(),
        },
        PendingAction::Action(ActionKind::Shop) => PromptChallenge {
            label: "Shop".to_string(),
            instruction: "type the supply command".to_string(),
            expected: "buy supplies".to_string(),
        },
        PendingAction::Action(ActionKind::Relax) => PromptChallenge {
            label: "Relax".to_string(),
            instruction: "type the park command".to_string(),
            expected: format!("relax {}", pick(&["park", "bench", "garden", "shade"], 2)),
        },
        PendingAction::Action(ActionKind::Shower) => PromptChallenge {
            label: "Shower".to_string(),
            instruction: "type the clean up command".to_string(),
            expected: "wash clean".to_string(),
        },
        PendingAction::Action(ActionKind::Chat) => PromptChallenge {
            label: "Chat".to_string(),
            instruction: format!("greet {}", subject),
            expected: format!("hello {}", subject),
        },
        PendingAction::Action(ActionKind::Exercise) => PromptChallenge {
            label: "Exercise".to_string(),
            instruction: "type the training command".to_string(),
            expected: "train cardio".to_string(),
        },
        PendingAction::Action(ActionKind::Meditate) => PromptChallenge {
            label: "Meditate".to_string(),
            instruction: "type the focus words".to_string(),
            expected: "breathe stay calm".to_string(),
        },
        PendingAction::Action(ActionKind::Bank) => PromptChallenge {
            label: "Bank".to_string(),
            instruction: "type the banking command".to_string(),
            expected: "bank open".to_string(),
        },
        PendingAction::Action(ActionKind::UseItem(ItemKind::Coffee)) => PromptChallenge {
            label: "Coffee".to_string(),
            instruction: "type the coffee action".to_string(),
            expected: "drink coffee".to_string(),
        },
        PendingAction::Action(ActionKind::UseItem(ItemKind::Vitamins)) => PromptChallenge {
            label: "Vitamins".to_string(),
            instruction: "type the vitamin action".to_string(),
            expected: "take vitamins".to_string(),
        },
        PendingAction::Action(ActionKind::UseItem(ItemKind::Books)) => PromptChallenge {
            label: "Books".to_string(),
            instruction: "type the reading action".to_string(),
            expected: "read book".to_string(),
        },
        PendingAction::Action(ActionKind::UseItem(ItemKind::Ingredient)) => PromptChallenge {
            label: "Ingredient".to_string(),
            instruction: "type the bag check".to_string(),
            expected: "check ingredient".to_string(),
        },
        PendingAction::Action(ActionKind::UseItem(ItemKind::GiftBox)) => PromptChallenge {
            label: "Gift Box".to_string(),
            instruction: "type the gift action".to_string(),
            expected: "open gift".to_string(),
        },
        PendingAction::Action(ActionKind::UseItem(ItemKind::Smoothie)) => PromptChallenge {
            label: "Smoothie".to_string(),
            instruction: "type the smoothie action".to_string(),
            expected: "drink smoothie".to_string(),
        },
        PendingAction::Action(ActionKind::Hobby(HobbyKind::Painting)) => PromptChallenge {
            label: "Painting".to_string(),
            instruction: "type the art command".to_string(),
            expected: "paint canvas".to_string(),
        },
        PendingAction::Action(ActionKind::Hobby(HobbyKind::Gaming)) => PromptChallenge {
            label: "Gaming".to_string(),
            instruction: "type the game command".to_string(),
            expected: "play game".to_string(),
        },
        PendingAction::Action(ActionKind::Hobby(HobbyKind::Music)) => PromptChallenge {
            label: "Music".to_string(),
            instruction: "type the music command".to_string(),
            expected: "play music".to_string(),
        },
        PendingAction::Action(ActionKind::StudyCourse) => PromptChallenge {
            label: "Study".to_string(),
            instruction: "type the study command".to_string(),
            expected: "study notes".to_string(),
        },
        PendingAction::Action(ActionKind::FeedPet) => PromptChallenge {
            label: "Feed Pet".to_string(),
            instruction: format!("type the pet command for {}", subject),
            expected: format!("feed {}", subject),
        },
        PendingAction::Action(ActionKind::ThrowParty) => PromptChallenge {
            label: "Party".to_string(),
            instruction: "type the host command".to_string(),
            expected: "host party".to_string(),
        },
        PendingAction::Action(ActionKind::BuyTransport) => PromptChallenge {
            label: "Transport".to_string(),
            instruction: "type the garage command".to_string(),
            expected: "visit garage".to_string(),
        },
        PendingAction::Action(ActionKind::GymSession) => PromptChallenge {
            label: "Gym".to_string(),
            instruction: "type the gym command".to_string(),
            expected: "lift strong".to_string(),
        },
        PendingAction::Action(ActionKind::Cafe) => PromptChallenge {
            label: "Cafe".to_string(),
            instruction: "type the cafe order".to_string(),
            expected: format!("order {}", pick(DRINK_WORDS, 3)),
        },
        PendingAction::Action(ActionKind::Clinic) => PromptChallenge {
            label: "Clinic".to_string(),
            instruction: "type the health command".to_string(),
            expected: "check health".to_string(),
        },
        PendingAction::Action(ActionKind::EnterVehicle) => PromptChallenge {
            label: "Vehicle".to_string(),
            instruction: "type the engine command".to_string(),
            expected: "start engine".to_string(),
        },
        PendingAction::Action(ActionKind::AdoptPet(kind)) => PromptChallenge {
            label: "Adopt Pet".to_string(),
            instruction: "type the adoption command".to_string(),
            expected: format!("adopt {}", kind.label().to_lowercase()),
        },
        PendingAction::Action(ActionKind::Craft) => PromptChallenge {
            label: "Craft".to_string(),
            instruction: "type the crafting command".to_string(),
            expected: "craft item".to_string(),
        },
        PendingAction::Action(ActionKind::RentUnit(id)) => PromptChallenge {
            label: format!("Rent Apt {}", id),
            instruction: "type the rental command".to_string(),
            expected: "sign lease".to_string(),
        },
        PendingAction::Action(ActionKind::GasUp) => PromptChallenge {
            label: "Gas Up".to_string(),
            instruction: "type the fuel command".to_string(),
            expected: "fill tank".to_string(),
        },
        PendingAction::Action(ActionKind::RepairVehicle) => PromptChallenge {
            label: "Repair".to_string(),
            instruction: "type the repair command".to_string(),
            expected: "fix vehicle".to_string(),
        },
        PendingAction::Action(ActionKind::DentalVisit) => PromptChallenge {
            label: "Dental".to_string(),
            instruction: "type the dental command".to_string(),
            expected: "open wide".to_string(),
        },
        PendingAction::Action(ActionKind::EyeExam) => PromptChallenge {
            label: "Eye Exam".to_string(),
            instruction: "type the exam command".to_string(),
            expected: "read chart".to_string(),
        },
        PendingAction::Action(ActionKind::ComputerLab) => PromptChallenge {
            label: "Computer Lab".to_string(),
            instruction: "type the login command".to_string(),
            expected: "study notes".to_string(),
        },
        PendingAction::Action(ActionKind::PrintShop) => PromptChallenge {
            label: "Print".to_string(),
            instruction: "type the print command".to_string(),
            expected: "print doc".to_string(),
        },
        PendingAction::Gift => PromptChallenge {
            label: "Gift".to_string(),
            instruction: format!("give a gift to {}", subject),
            expected: format!("give {} gift", subject),
        },
        PendingAction::Bank(slot) => {
            let (label, expected) = match slot {
                1 => ("Deposit", "bank deposit"),
                2 => ("Withdraw", "bank withdraw"),
                3 => ("Half Deposit", "bank half"),
                4 => ("Loan", "bank loan"),
                5 => ("Repay", "bank repay"),
                6 => ("Invest", "bank invest"),
                7 => ("Medium Invest", "bank medium"),
                8 => ("Cash Out", "bank cashout"),
                _ => ("Insurance", "bank insure"),
            };
            PromptChallenge {
                label: label.to_string(),
                instruction: "type the banking action".to_string(),
                expected: expected.to_string(),
            }
        }
        PendingAction::Transport(slot) => {
            let (label, expected) = match slot {
                1 => ("Buy Bike", "buy bike"),
                2 => ("Buy Car", "buy car"),
                _ => ("Service", "service engine"),
            };
            PromptChallenge {
                label: label.to_string(),
                instruction: "type the transport action".to_string(),
                expected: expected.to_string(),
            }
        }
        PendingAction::Craft(slot) => {
            let (label, expected): (&str, String) = match slot {
                1 => ("Cook", format!("add {}", pick(INGREDIENT_WORDS, 4))),
                2 => ("Gift Box", "wrap ribbon".to_string()),
                _ => (
                    "Smoothie",
                    format!("blend {}", pick(&["berry", "banana", "mango", "melon"], 5)),
                ),
            };
            PromptChallenge {
                label: label.to_string(),
                instruction: "type the crafting action".to_string(),
                expected,
            }
        }
        PendingAction::Festival(slot) => {
            let (label, expected) = match slot {
                1 => ("Festival", "join festival"),
                2 => ("Festival", "play festival"),
                3 => ("Festival", "enjoy festival"),
                _ => ("Festival", "redeem token"),
            };
            PromptChallenge {
                label: label.to_string(),
                instruction: "type the festival action".to_string(),
                expected: expected.to_string(),
            }
        }
    }
}

fn normalize_prompt_text(input: &str) -> String {
    input
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn append_prompt_char(keys: &ButtonInput<KeyCode>, buffer: &mut String, kc: KeyCode, ch: char) {
    if keys.just_pressed(kc) && buffer.len() < 48 {
        buffer.push(ch);
    }
}

fn collect_prompt_text(keys: &ButtonInput<KeyCode>, buffer: &mut String) {
    let letters = [
        (KeyCode::KeyA, 'a'),
        (KeyCode::KeyB, 'b'),
        (KeyCode::KeyC, 'c'),
        (KeyCode::KeyD, 'd'),
        (KeyCode::KeyE, 'e'),
        (KeyCode::KeyF, 'f'),
        (KeyCode::KeyG, 'g'),
        (KeyCode::KeyH, 'h'),
        (KeyCode::KeyI, 'i'),
        (KeyCode::KeyJ, 'j'),
        (KeyCode::KeyK, 'k'),
        (KeyCode::KeyL, 'l'),
        (KeyCode::KeyM, 'm'),
        (KeyCode::KeyN, 'n'),
        (KeyCode::KeyO, 'o'),
        (KeyCode::KeyP, 'p'),
        (KeyCode::KeyQ, 'q'),
        (KeyCode::KeyR, 'r'),
        (KeyCode::KeyS, 's'),
        (KeyCode::KeyT, 't'),
        (KeyCode::KeyU, 'u'),
        (KeyCode::KeyV, 'v'),
        (KeyCode::KeyW, 'w'),
        (KeyCode::KeyX, 'x'),
        (KeyCode::KeyY, 'y'),
        (KeyCode::KeyZ, 'z'),
    ];
    for (kc, ch) in letters {
        append_prompt_char(keys, buffer, kc, ch);
    }
    if keys.just_pressed(KeyCode::Space) && !buffer.ends_with(' ') && !buffer.is_empty() {
        buffer.push(' ');
    }
}

fn begin_action_prompt(
    prompt: &mut ActionPrompt,
    pending: PendingAction,
    target: Entity,
    gt: &GameTime,
    skills: &Skills,
    pet_name: &str,
    notif: &mut Notification,
) {
    let seed = gt.day.wrapping_mul(37).wrapping_add(gt.hours as u32);
    let challenge = build_prompt_challenge(&pending, seed, pet_name);
    prompt.active = true;
    prompt.buffer.clear();
    prompt.label = challenge.label;
    prompt.instruction = challenge.instruction;
    prompt.expected = challenge.expected;
    prompt.retries_left = action_prompt_retries(skills.career);
    prompt.pending = Some(pending);
    prompt.target = Some(target);
    notif.push(
        format!("{} challenge - type: {}", prompt.label, prompt.expected),
        4.,
    );
}

fn handle_action_prompt_input(
    keys: &ButtonInput<KeyCode>,
    prompt: &mut ActionPrompt,
    notif: &mut Notification,
) -> Option<(PendingAction, Option<Entity>)> {
    if !prompt.active {
        return None;
    }

    let tries_label = if prompt.retries_left == 1 {
        "try"
    } else {
        "tries"
    };
    notif.message = format!(
        "{} - {} | target phrase: {} | {} {} left | {}_",
        prompt.label,
        prompt.instruction,
        prompt.expected,
        prompt.retries_left,
        tries_label,
        prompt.buffer
    );
    notif.timer = 1.0;

    if keys.just_pressed(KeyCode::Escape) {
        prompt.clear();
        notif.push("Typing challenge cancelled.", 1.5);
        return None;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        prompt.buffer.pop();
        return None;
    }

    collect_prompt_text(keys, &mut prompt.buffer);

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        let attempt = normalize_prompt_text(&prompt.buffer);
        if attempt == normalize_prompt_text(&prompt.expected) {
            let label = prompt.label.clone();
            let pending = prompt.pending.take();
            let target = prompt.target.take();
            prompt.clear();
            notif.push(format!("{} confirmed.", label), 1.5);
            return pending.map(|next| (next, target));
        }

        if prompt.retries_left > 1 {
            prompt.retries_left -= 1;
            prompt.buffer.clear();
            notif.push(format!("Not quite. Try again: {}", prompt.expected), 2.5);
        } else {
            let label = prompt.label.clone();
            prompt.clear();
            notif.push(format!("{} failed. Action blocked.", label), 2.5);
        }
    }

    None
}

fn pending_action_from_input(
    inter: &Interactable,
    festival_active: bool,
    pe: bool,
    pg: bool,
    p1: bool,
    p2: bool,
    p3: bool,
    p4: bool,
    p5: bool,
    p6: bool,
    p7: bool,
    p8: bool,
    p9: bool,
) -> Option<PendingAction> {
    if pg && matches!(&inter.action, ActionKind::Chat) {
        return Some(PendingAction::Gift);
    }

    if matches!(&inter.action, ActionKind::Bank) {
        let slot = if p1 {
            Some(1)
        } else if p2 {
            Some(2)
        } else if p3 {
            Some(3)
        } else if p4 {
            Some(4)
        } else if p5 {
            Some(5)
        } else if p6 {
            Some(6)
        } else if p7 {
            Some(7)
        } else if p8 {
            Some(8)
        } else if p9 {
            Some(9)
        } else {
            None
        };
        if let Some(slot) = slot {
            return Some(PendingAction::Bank(slot));
        }
    }

    if matches!(&inter.action, ActionKind::BuyTransport) {
        let slot = if p1 {
            Some(1)
        } else if p2 {
            Some(2)
        } else if p3 {
            Some(3)
        } else {
            None
        };
        if let Some(slot) = slot {
            return Some(PendingAction::Transport(slot));
        }
    }

    if matches!(&inter.action, ActionKind::Craft) {
        let slot = if p1 {
            Some(1)
        } else if p2 {
            Some(2)
        } else if p3 {
            Some(3)
        } else {
            None
        };
        if let Some(slot) = slot {
            return Some(PendingAction::Craft(slot));
        }
    }

    if festival_active && matches!(&inter.action, ActionKind::Relax) {
        let slot = if p1 {
            Some(1)
        } else if p2 {
            Some(2)
        } else if p3 {
            Some(3)
        } else if p4 {
            Some(4)
        } else {
            None
        };
        if let Some(slot) = slot {
            return Some(PendingAction::Festival(slot));
        }
    }

    pe.then_some(PendingAction::Action(inter.action.clone()))
}

// ── Extracted shortcut handlers ───────────────────────────────────────────────

/// Handle bank sub-menu key presses (1-9). Returns `true` if a key was handled.
fn handle_bank_keys(
    p1: bool,
    p2: bool,
    p3: bool,
    p4: bool,
    p5: bool,
    p6: bool,
    p7: bool,
    p8: bool,
    p9: bool,
    gt: &mut GameTime,
    stats: &mut PlayerStats,
    notif: &mut Notification,
    bank_input: &mut BankInput,
    invest: &mut Investment,
    crisis: &mut CrisisState,
) -> bool {
    if p1 {
        bank_input.active = true;
        bank_input.kind = BankInputKind::Deposit;
        bank_input.buffer.clear();
        notif.push("Deposit amount: $_ [Enter]=confirm [Esc]=cancel", 10.);
        return true;
    }
    if p2 {
        bank_input.active = true;
        bank_input.kind = BankInputKind::Withdraw;
        bank_input.buffer.clear();
        notif.push("Withdraw amount: $_ [Enter]=confirm [Esc]=cancel", 10.);
        return true;
    }
    if p3 {
        let half = (stats.money / 2.).floor().max(0.);
        let msg =
            if let Some((new_money, new_savings)) = try_deposit(stats.money, stats.savings, half) {
                gt.advance_hours(0.25);
                stats.money = new_money;
                stats.savings = new_savings;
                format!("Deposited ${:.0}. Savings: ${:.0}", half, stats.savings)
            } else {
                "Nothing to deposit.".to_string()
            };
        notif.push(msg, 2.);
        stats.cooldown = 0.5;
        return true;
    }
    if p4 {
        if stats.loan < 200. {
            gt.advance_hours(0.25);
            stats.loan += 100.;
            stats.money += 100.;
            stats.stress = (stats.stress + 10.).min(100.);
            notif.message = format!("Took $100 loan (8%/day). Total: ${:.0}", stats.loan);
        } else {
            notif.message = "Loan limit reached! Repay first.".to_string();
        }
        notif.flush_message(3.);
        stats.cooldown = 0.5;
        return true;
    }
    if p5 {
        let pay = stats.loan.min(50.).min(stats.money);
        if pay > 0. {
            gt.advance_hours(0.25);
            stats.loan -= pay;
            stats.money -= pay;
            stats.stress = (stats.stress - 5.).max(0.);
            notif.message = format!("Repaid ${:.0}. Remaining: ${:.0}", pay, stats.loan);
        } else {
            notif.message = "Nothing to repay or no cash.".to_string();
        }
        notif.flush_message(2.5);
        stats.cooldown = 0.5;
        return true;
    }
    if p6 {
        if stats.can_afford(50.) {
            gt.advance_hours(0.25);
            stats.money -= 50.;
            invest.amount += 50.;
            if invest.risk == 0 {
                invest.risk = 1;
                invest.daily_return_rate = 0.04;
            }
            notif.message = format!("Invested $50 (Low risk). Portfolio: ${:.0}", invest.amount);
        } else {
            notif.message = "Need $50 to invest!".to_string();
        }
        notif.flush_message(3.);
        stats.cooldown = 0.5;
        return true;
    }
    if p7 {
        if stats.can_afford(50.) {
            gt.advance_hours(0.25);
            stats.money -= 50.;
            invest.amount += 50.;
            invest.risk = 2;
            invest.daily_return_rate = 0.10;
            notif.message = format!(
                "Invested $50 (Medium risk). Portfolio: ${:.0}",
                invest.amount
            );
        } else {
            notif.message = "Need $50!".to_string();
        }
        notif.flush_message(3.);
        stats.cooldown = 0.5;
        return true;
    }
    if p8 {
        if invest.amount > 0. {
            gt.advance_hours(0.25);
            let amt = invest.amount;
            stats.money += amt;
            invest.amount = 0.;
            invest.risk = 0;
            invest.daily_return_rate = 0.;
            notif.message = format!("Withdrew entire investment: +${:.0}", amt);
        } else {
            notif.message = "No investment to withdraw.".to_string();
        }
        notif.flush_message(3.);
        stats.cooldown = 0.5;
        return true;
    }
    if p9 {
        if crisis.has_insurance {
            notif.message = format!("Already insured! ({} days left)", crisis.insurance_days);
        } else if stats.can_afford(75.) {
            gt.advance_hours(0.25);
            stats.money -= 75.;
            crisis.has_insurance = true;
            crisis.insurance_days = 30;
            notif.message = "Bought insurance for 30 days! Crisis damage halved.".to_string();
        } else {
            notif.message = "Need $75 for insurance!".to_string();
        }
        notif.flush_message(3.);
        stats.cooldown = 0.5;
        return true;
    }
    false
}

/// Handle transport sub-menu key presses (1-3). Returns `true` if a key was handled.
fn handle_transport_keys(
    p1: bool,
    p2: bool,
    p3: bool,
    gt: &mut GameTime,
    stats: &mut PlayerStats,
    notif: &mut Notification,
    transport: &mut Transport,
) -> bool {
    if p1 {
        if transport.kind == TransportKind::Bike || transport.kind == TransportKind::Car {
            notif.message = "Already have a vehicle!".to_string();
        } else if stats.savings >= 80. {
            gt.advance_hours(0.5);
            stats.savings -= 80.;
            transport.kind = TransportKind::Bike;
            transport.work_uses = 0;
            transport.maintenance_due = false;
            notif.message =
                "Bought a Bicycle! Work pays 1.1x bonus. Service every 5 uses.".to_string();
        } else {
            notif.message = "Need $80 in savings for a Bike!".to_string();
        }
        notif.flush_message(3.);
        stats.cooldown = 0.5;
        return true;
    }
    if p2 {
        if transport.kind == TransportKind::Car {
            notif.message = "Already have a Car! Find it near the Garage.".to_string();
        } else if stats.savings >= 300. {
            gt.advance_hours(0.5);
            stats.savings -= 300.;
            transport.kind = TransportKind::Car;
            transport.work_uses = 0;
            transport.maintenance_due = false;
            notif.message =
                "Bought a Car! Find it parked near the Garage. Work pays 1.2x.".to_string();
        } else {
            notif.message = "Need $300 in savings for a Car!".to_string();
        }
        notif.flush_message(3.);
        stats.cooldown = 0.5;
        return true;
    }
    if p3 {
        if !transport.kind.is_vehicle() {
            notif.message = "No vehicle to service!".to_string();
        } else if !transport.maintenance_due {
            notif.message = format!(
                "{} is in good shape. ({} uses, service needed at 5).",
                transport.kind.label(),
                transport.work_uses
            );
        } else if stats.can_afford(15.) {
            gt.advance_hours(0.25);
            stats.money -= 15.;
            transport.maintenance_due = false;
            transport.work_uses = 0;
            notif.message = format!(
                "Serviced {}! Pay bonus restored. ($15 paid)",
                transport.kind.label()
            );
        } else {
            notif.message = "Need $15 cash for vehicle service!".to_string();
        }
        notif.flush_message(3.);
        stats.cooldown = 0.5;
        return true;
    }
    false
}

/// Handle craft sub-menu key presses (1-3). Returns `true` if a key was handled.
fn handle_craft_keys(
    p1: bool,
    p2: bool,
    p3: bool,
    gt: &mut GameTime,
    stats: &mut PlayerStats,
    notif: &mut Notification,
    inv: &mut Inventory,
    skills: &mut Skills,
    gs: &mut GameState,
    quest_board: &mut QuestBoard,
) -> bool {
    if p1 {
        gt.advance_hours(0.5);
        if inv.ingredient >= 2 {
            inv.ingredient -= 2;
            stats.meals += 3;
            let gain = 0.20 * stats.skill_gain_mult();
            skills.cooking = (skills.cooking + gain).min(5.);
            gs.eat_today += 1;
            quest_board.crafted_today += 1;
            gs.total_crafted += 1;
            notif.push(
                format!(
                    "Cooked a meal! +3 meals +{:.2} Cooking. ({}x ingredients left)",
                    gain, inv.ingredient
                ),
                3.,
            );
        } else {
            notif.push(
                format!("Need 2 ingredients to cook. (have {}x)", inv.ingredient),
                2.5,
            );
        }
        stats.cooldown = 0.5;
        return true;
    }
    if p2 {
        gt.advance_hours(0.5);
        if inv.ingredient >= 1 && stats.can_afford(5.) {
            inv.ingredient -= 1;
            stats.money -= 5.;
            inv.gift_box += 1;
            quest_board.crafted_today += 1;
            gs.total_crafted += 1;
            notif.push(
                format!(
                    "Crafted a Gift Box! ({}x). Give it to an NPC with [G].",
                    inv.gift_box
                ),
                3.,
            );
        } else if inv.ingredient < 1 {
            notif.push(
                format!(
                    "Need 1 ingredient for a gift box. (have {}x)",
                    inv.ingredient
                ),
                2.5,
            );
        } else {
            notif.push("Need $5 to craft a gift box.", 2.5);
        }
        stats.cooldown = 0.5;
        return true;
    }
    if p3 {
        gt.advance_hours(0.5);
        if inv.ingredient >= 2 {
            inv.ingredient -= 2;
            inv.smoothie += 1;
            let gain = 0.10 * stats.skill_gain_mult();
            skills.cooking = (skills.cooking + gain).min(5.);
            quest_board.crafted_today += 1;
            gs.total_crafted += 1;
            notif.push(
                format!(
                    "Blended a Smoothie! ({}x). Use at home for +40 Energy +10 Health.",
                    inv.smoothie
                ),
                3.,
            );
        } else {
            notif.push(
                format!(
                    "Need 2 ingredients for a smoothie. (have {}x)",
                    inv.ingredient
                ),
                2.5,
            );
        }
        stats.cooldown = 0.5;
        return true;
    }
    false
}

// ── Extracted match-arm handlers ──────────────────────────────────────────────

/// ActionKind::Work - office employment with pay modifiers.
fn handle_work(
    gt: &mut GameTime,
    stats: &mut PlayerStats,
    skills: &mut Skills,
    gs: &mut GameState,
    notif: &mut Notification,
    streak: &mut WorkStreak,
    housing: &HousingTier,
    conds: &Conditions,
    transport: &mut Transport,
    rep: &mut Reputation,
    pm: &PlayerMovement,
    settings: &GameSettings,
    crisis: &CrisisState,
) {
    if crisis.is_laid_off() {
        notif.push(
            format!(
                "Laid off! {} day(s) remaining. Can't work yet.",
                crisis.days_left
            ),
            3.,
        );
        return;
    }
    let pos = pm.prev_position;
    if pos.x < OFFICE_LEFT || pos.x > OFFICE_RIGHT || pos.y < OFFICE_BOTTOM || pos.y > OFFICE_TOP {
        notif.push("You need to be at the office to work!", 2.5);
        return;
    }
    if stats.energy < 15. {
        notif.push("Too tired to work!", 2.);
        return;
    }
    if stats.health < 20. {
        notif.push("Too sick to work!", 2.);
        return;
    }
    if stats.stress > 90. {
        notif.push("Too stressed! Meditate or relax first.", 3.);
        return;
    }
    let mood = Mood::from_happiness(stats.happiness);
    let (time_mult, time_tag) = gt.work_time_tag();
    let weekend = if gt.is_weekend() { 1.5 } else { 1.0 };
    let burnout_mult = conds.work_pay_mult();
    let transport_mult = transport.effective_work_bonus();
    let rep_mult = rep.work_mult();
    let health_mult = health_work_mult(stats.health);
    let earned = skills.work_pay(streak.days)
        * mood.work_mult()
        * time_mult
        * weekend
        * stats.stress_work_mult()
        * stats.loan_penalty()
        * burnout_mult
        * transport_mult
        * rep_mult
        * health_mult
        * settings.difficulty.economy_mult();
    if transport.kind.is_vehicle() {
        transport.work_uses += 1;
        if transport.work_uses >= 5 && !transport.maintenance_due {
            transport.maintenance_due = true;
            notif.push(
                "Vehicle needs service! Pay bonus suspended until repaired at Garage [3] $15.",
                6.,
            );
        }
    }
    stats.money += earned;
    stats.energy = (stats.energy - 8.).max(0.);
    stats.happiness = (stats.happiness - 5.).max(0.);
    stats.stress = (stats.stress + 5.).min(100.);
    skills.career = (skills.career + 0.15 * stats.skill_gain_mult()).min(5.);
    gs.work_today += 1;
    gs.money_earned_today += earned;
    streak.worked_today = true;
    streak.days += 1;
    rep.score = (rep.score + 0.5).min(100.);
    stats.cooldown = 2.;
    let we = if gt.is_weekend() {
        " [Weekend 1.5x]"
    } else {
        ""
    };
    let st = if streak.days >= 3 {
        format!(" [Streak {}d]", streak.days)
    } else {
        String::new()
    };
    let bt = if conds.burnout { " [Burnout -30%]" } else { "" };
    let tr = if transport.maintenance_due {
        " [Vehicle broken -fix at Garage]"
    } else {
        match transport.kind {
            TransportKind::Bike => " [Bike +10%]",
            TransportKind::Car => " [Car +20%]",
            _ => "",
        }
    };
    let rp = if rep.score < 20. {
        " [Low rep -10%]"
    } else if rep.score >= 60. {
        " [Rep +10%]"
    } else {
        ""
    };
    let hl = if stats.health < 25. {
        " [Poor Health -25%]"
    } else if stats.health < 50. {
        " [Unwell -10%]"
    } else {
        ""
    };
    if gs.work_today == 1 && *housing == HousingTier::Unhoused {
        notif.push(
            format!(
                "Earned ${:.0}! Head to the Bank (SW) and deposit to rent your apartment.",
                earned
            ),
            5.,
        );
    } else if *housing == HousingTier::Unhoused && stats.can_afford(90.) {
        notif.push(
            format!(
                "${:.0} cash - enough for the apartment! Bank is SW.",
                stats.money
            ),
            5.,
        );
    } else {
        notif.push(
            format!(
                "[{}]{}{}{}{}{}{}{} Earned ${:.0}!",
                skills.career_rank(),
                we,
                time_tag,
                st,
                bt,
                tr,
                rp,
                hl,
                earned
            ),
            2.5,
        );
    }
}

/// ActionKind::Shop - grocery store with numbered sub-items.
fn handle_shop(
    pe: bool,
    p1: bool,
    p2: bool,
    p3: bool,
    p4: bool,
    stats: &mut PlayerStats,
    inv: &mut Inventory,
    notif: &mut Notification,
) {
    if p1 {
        if stats.can_afford(5.) && inv.coffee < 9 {
            stats.money -= 5.;
            inv.coffee += 1;
            inv.coffee_age = 0;
            notif.message = format!("Bought coffee. ({}x, fresh)", inv.coffee);
        } else if inv.coffee >= 9 {
            notif.message = "Coffee stack full (9)!".to_string();
        } else {
            notif.message = "Need $5 for coffee.".to_string();
        }
        notif.flush_message(2.);
        stats.cooldown = 0.5;
        return;
    }
    if p2 {
        if stats.can_afford(8.) && inv.vitamins < 9 {
            stats.money -= 8.;
            inv.vitamins += 1;
            notif.message = format!("Bought vitamins. ({}x)", inv.vitamins);
        } else if inv.vitamins >= 9 {
            notif.message = "Vitamins stack full (9)!".to_string();
        } else {
            notif.message = "Need $8 for vitamins.".to_string();
        }
        notif.flush_message(2.);
        stats.cooldown = 0.5;
        return;
    }
    if p3 {
        if stats.can_afford(12.) && inv.books < 9 {
            stats.money -= 12.;
            inv.books += 1;
            notif.message = format!("Bought book. ({}x)", inv.books);
        } else if inv.books >= 9 {
            notif.message = "Books stack full (9)!".to_string();
        } else {
            notif.message = "Need $12 for a book.".to_string();
        }
        notif.flush_message(2.);
        stats.cooldown = 0.5;
        return;
    }
    if p4 {
        if stats.can_afford(8.) && inv.ingredient < 9 {
            stats.money -= 8.;
            inv.ingredient += 1;
            notif.message = format!("Bought ingredients. ({}x)", inv.ingredient);
        } else if inv.ingredient >= 9 {
            notif.message = "Ingredients stack full (9)!".to_string();
        } else {
            notif.message = "Need $8 for ingredients.".to_string();
        }
        notif.flush_message(2.);
        stats.cooldown = 0.5;
        return;
    }
    if pe {
        if stats.can_afford(15.) {
            stats.money -= 15.;
            stats.meals += 3;
            stats.cooldown = 1.;
            notif.push(
                "Groceries: +3 meals ($15). [1]☕$5 [2]💊$8 [3]📚$12 [4]🥕$8",
                3.,
            );
        } else {
            notif.push(
                "Need $15 for groceries. [1]☕$5 [2]💊$8 [3]📚$12 [4]🥕$8",
                2.,
            );
        }
    }
}

/// ActionKind::Relax - park relaxation + festival activities.
fn handle_relax(
    p1: bool,
    p2: bool,
    p3: bool,
    p4: bool,
    gt: &mut GameTime,
    stats: &mut PlayerStats,
    skills: &mut Skills,
    gs: &mut GameState,
    inv: &mut Inventory,
    notif: &mut Notification,
    friendship: &mut NpcFriendship,
    festival: &mut FestivalState,
    weather: &WeatherKind,
    season: &Season,
    hobbies: &mut Hobbies,
    rep: &mut Reputation,
    pet: &Pet,
) {
    // ── Festival activities at Park ───────────────────────────────────────
    if festival.is_active() && (p1 || p2 || p3 || p4) {
        if p4 {
            if festival.tokens >= 10 {
                festival.tokens -= 10;
                inv.gift_box += 1;
                inv.ingredient += 2;
                stats.money += 25.;
                notif.push("Redeemed 10 tokens: Gift Box + 2 Ingredients + $25!", 3.);
            } else {
                notif.push(
                    format!("Need 10 tokens to redeem (have {}).", festival.tokens),
                    2.,
                );
            }
            stats.cooldown = 0.5;
            return;
        }
        if festival.activities_today >= 3 {
            notif.push(
                "You've done 3 festival activities today! Come back tomorrow.",
                2.,
            );
            stats.cooldown = 0.5;
            return;
        }
        let Some(kind) = festival.active.clone() else {
            return;
        };
        match (&kind, p1, p2) {
            (FestivalKind::SpringFair, true, _) => {
                if !stats.can_afford(5.) {
                    notif.push("Need $5 for Flower Crown!", 2.);
                    stats.cooldown = 0.5;
                    return;
                }
                stats.money -= 5.;
                stats.happiness = (stats.happiness + 10.).min(100.);
                skills.social = (skills.social + 0.15).min(5.);
                festival.tokens += 2;
                festival.spring_attended = true;
                notif.push("Made a Flower Crown! +10hap +0.15social +2 tokens", 3.);
            }
            (FestivalKind::SpringFair, _, true) => {
                if stats.energy < 10. {
                    notif.push("Too tired for the Dance Contest!", 2.);
                    stats.cooldown = 0.5;
                    return;
                }
                stats.energy -= 10.;
                stats.happiness = (stats.happiness + 15.).min(100.);
                skills.fitness = (skills.fitness + 0.2).min(5.);
                festival.tokens += 3;
                festival.spring_attended = true;
                notif.push("Danced your heart out! +15hap +0.2fit +3 tokens", 3.);
            }
            (FestivalKind::SpringFair, _, _) => {
                if pet.has_pet {
                    rep.score = (rep.score + 5.).min(100.);
                    festival.tokens += 2;
                    festival.spring_attended = true;
                    notif.push(format!("{} won a ribbon! +5rep +2 tokens", pet.name), 3.);
                } else {
                    notif.push("No pet to enter the Pet Parade!", 2.);
                    stats.cooldown = 0.5;
                    return;
                }
            }
            (FestivalKind::SummerBBQ, true, _) => {
                if !stats.can_afford(10.) {
                    notif.push("Need $10 for the Grill-Off!", 2.);
                    stats.cooldown = 0.5;
                    return;
                }
                stats.money -= 10.;
                stats.hunger = (stats.hunger - 20.).max(0.);
                skills.cooking = (skills.cooking + 0.2).min(5.);
                festival.tokens += 2;
                festival.summer_attended = true;
                notif.push("Grilled up a feast! -20hunger +0.2cook +2 tokens", 3.);
            }
            (FestivalKind::SummerBBQ, _, true) => {
                if stats.energy < 15. {
                    notif.push("Too tired for the Swim Race!", 2.);
                    stats.cooldown = 0.5;
                    return;
                }
                stats.energy -= 15.;
                skills.fitness = (skills.fitness + 0.3).min(5.);
                stats.stress = (stats.stress - 10.).max(0.);
                festival.tokens += 3;
                festival.summer_attended = true;
                notif.push("Won the Swim Race! +0.3fit -10stress +3 tokens", 3.);
            }
            (FestivalKind::SummerBBQ, _, _) => {
                stats.happiness = (stats.happiness + 20.).min(100.);
                stats.stress = (stats.stress - 15.).max(0.);
                festival.tokens += 1;
                festival.summer_attended = true;
                notif.push("Watched the fireworks! +20hap -15stress +1 token", 3.);
            }
            (FestivalKind::AutumnHarvest, true, _) => {
                if stats.energy < 5. {
                    notif.push("Too tired for Pumpkin Carving!", 2.);
                    stats.cooldown = 0.5;
                    return;
                }
                stats.energy -= 5.;
                hobbies.painting = (hobbies.painting + 0.2).min(5.);
                stats.happiness = (stats.happiness + 10.).min(100.);
                festival.tokens += 2;
                festival.autumn_attended = true;
                notif.push("Carved a pumpkin! +0.2art +10hap +2 tokens", 3.);
            }
            (FestivalKind::AutumnHarvest, _, true) => {
                inv.ingredient += 3;
                skills.fitness = (skills.fitness + 0.1).min(5.);
                festival.tokens += 2;
                festival.autumn_attended = true;
                notif.push("Picked apples! +3 ingredients +0.1fit +2 tokens", 3.);
            }
            (FestivalKind::AutumnHarvest, _, _) => {
                if !stats.can_afford(15.) {
                    notif.push("Need $15 for the Harvest Feast!", 2.);
                    stats.cooldown = 0.5;
                    return;
                }
                stats.money -= 15.;
                stats.hunger = (stats.hunger - 30.).max(0.);
                stats.happiness = (stats.happiness + 15.).min(100.);
                skills.cooking = (skills.cooking + 0.1).min(5.);
                festival.tokens += 3;
                festival.autumn_attended = true;
                notif.push("Harvest Feast! -30hunger +15hap +0.1cook +3 tokens", 3.);
            }
            (FestivalKind::WinterGala, true, _) => {
                if stats.energy < 10. {
                    notif.push("Too tired for Ice Skating!", 2.);
                    stats.cooldown = 0.5;
                    return;
                }
                stats.energy -= 10.;
                skills.fitness = (skills.fitness + 0.2).min(5.);
                stats.happiness = (stats.happiness + 15.).min(100.);
                festival.tokens += 2;
                festival.winter_attended = true;
                notif.push("Ice Skating! +0.2fit +15hap +2 tokens", 3.);
            }
            (FestivalKind::WinterGala, _, true) => {
                if !stats.can_afford(20.) {
                    notif.push("Need $20 for the Gift Exchange!", 2.);
                    stats.cooldown = 0.5;
                    return;
                }
                stats.money -= 20.;
                festival.tokens += 3;
                festival.winter_attended = true;
                for lvl in friendship.levels.values_mut() {
                    *lvl = (*lvl + 0.5).min(5.);
                }
                notif.push("Gift Exchange! +0.5 friendship to all NPCs +3 tokens", 3.);
            }
            (FestivalKind::WinterGala, _, _) => {
                if !stats.can_afford(25.) {
                    notif.push("Need $25 for the Charity Drive!", 2.);
                    stats.cooldown = 0.5;
                    return;
                }
                stats.money -= 25.;
                rep.score = (rep.score + 10.).min(100.);
                stats.stress = (stats.stress - 5.).max(0.);
                festival.tokens += 3;
                festival.winter_attended = true;
                notif.push("Charity Drive! +10rep -5stress +3 tokens", 3.);
            }
        }
        festival.activities_today += 1;
        festival.festivals_total += 1;
        gt.advance_hours(1.);
        stats.cooldown = 3.;
        return;
    }

    if weather.is_stormy() {
        notif.push("Stormy outside! Can't relax here.", 2.);
        return;
    }
    let gain = 20. * skills.social_bonus();
    let weather_bonus = weather.outdoor_hap_bonus();
    let season_bonus = season.current.outdoor_bonus();
    stats.happiness = (stats.happiness + gain + weather_bonus + season_bonus).min(100.);
    stats.energy = (stats.energy - 3.).max(0.);
    stats.stress = (stats.stress - 12.).max(0.);
    skills.fitness = (skills.fitness + 0.08).min(5.);
    gs.outdoor_done_today = true;
    stats.cooldown = 3.;
    let wb = if weather_bonus + season_bonus > 0. {
        format!(" [+{:.0} outdoor bonus]", weather_bonus + season_bonus)
    } else {
        String::new()
    };
    let fest_hint = if let Some(k) = festival.active.as_ref() {
        let acts = match k {
            FestivalKind::SpringFair => "[1]FlowerCrown [2]Dance [3]PetParade",
            FestivalKind::SummerBBQ => "[1]Grill-Off [2]SwimRace [3]Fireworks",
            FestivalKind::AutumnHarvest => "[1]Pumpkin [2]ApplePick [3]Feast",
            FestivalKind::WinterGala => "[1]Skate [2]GiftSwap [3]Charity",
        };
        format!(" {} {} [4]Redeem({}tok)", k.label(), acts, festival.tokens)
    } else {
        String::new()
    };
    notif.push(
        format!("Relaxed. +{:.0}hap, -Stress.{}{}", gain, wb, fest_hint),
        3.,
    );
}

/// ActionKind::Chat - NPC interaction and gifting.
fn handle_chat(
    pe: bool,
    pg: bool,
    entity: Entity,
    stats: &mut PlayerStats,
    skills: &mut Skills,
    gs: &mut GameState,
    inv: &mut Inventory,
    notif: &mut Notification,
    friendship: &mut NpcFriendship,
    rep: &mut Reputation,
    season: &Season,
    npc_data: Option<(&Npc, &NpcId)>,
) {
    let personality = npc_data
        .map(|(n, _)| n.personality)
        .unwrap_or(NpcPersonality::Neutral);
    let npc_name = npc_data.map(|(n, _)| n.name.as_str()).unwrap_or("them");
    if pg {
        let lvl = friendship.levels.get(&entity).copied().unwrap_or(0.);
        if friendship
            .gifted_today
            .get(&entity)
            .copied()
            .unwrap_or(false)
        {
            notif.push("Already gifted this NPC today!".to_string(), 2.);
            return;
        }
        let used_gift_box = inv.gift_box > 0;
        if lvl >= 2. && (used_gift_box || stats.can_afford(10.)) {
            if used_gift_box {
                inv.gift_box -= 1;
            } else {
                stats.money -= 10.;
            }
            let friend_gain = if used_gift_box { 0.6 } else { 0.3 };
            let f = friendship.levels.entry(entity).or_insert(0.);
            *f = (*f + friend_gain).min(5.);
            friendship.gifted_today.insert(entity, true);
            gs.total_gifts += 1;
            stats.cooldown = 2.;
            let gift_label = if used_gift_box { " (Gift Box!)" } else { "" };
            let (hap, health_gain, rep_gain, msg) = match personality {
                NpcPersonality::Cheerful => (
                    38.,
                    0.,
                    0.,
                    format!(
                        "Gift to {}! +38 Happiness (Cheerful){}.",
                        npc_name, gift_label
                    ),
                ),
                NpcPersonality::Wise => (
                    10.,
                    15.,
                    0.,
                    format!("Gift to {}! +15 Health (Wise){}.", npc_name, gift_label),
                ),
                NpcPersonality::Influential => (
                    10.,
                    0.,
                    15.,
                    format!("Gift to {}! +15 Rep (Influential){}.", npc_name, gift_label),
                ),
                NpcPersonality::Neutral => (
                    25.,
                    0.,
                    0.,
                    format!("Gift to {}! +25 Happiness{}.", npc_name, gift_label),
                ),
            };
            stats.happiness = (stats.happiness + hap).min(100.);
            stats.health = (stats.health + health_gain).min(100.);
            rep.score = (rep.score + rep_gain).clamp(0., 100.);
            notif.message = msg;
        } else if lvl < 2. {
            notif.message = "Need Acquaintance (friendship 2+) to gift!".to_string();
        } else {
            notif.message = "Need a Gift Box or $10 to gift.".to_string();
        }
        notif.flush_message(2.);
        return;
    }
    if !pe {
        return;
    }
    let f = friendship.levels.entry(entity).or_insert(0.);
    let chat_bonus = rep.chat_bonus() * season.current.social_mult();
    let gain_mult = (1. + (*f * 0.05)) * chat_bonus;
    let hap_mult = if personality == NpcPersonality::Cheerful {
        1.5
    } else {
        1.0
    };
    let skill_mult = if personality == NpcPersonality::Wise {
        1.5
    } else {
        1.0
    };
    let rep_friend_mult = if rep.score < 15. { 0.5 } else { 1.0 };
    let hap = 15. * skills.social_bonus() * gain_mult * hap_mult;
    *f = (*f + 0.30 * rep_friend_mult).min(5.);
    let lvl = *f;
    friendship.chatted_today.insert(entity, true);
    stats.happiness = (stats.happiness + hap).min(100.);
    stats.stress = (stats.stress - 5.).max(0.);
    skills.social = (skills.social + 0.15 * skill_mult * stats.skill_gain_mult()).min(5.);
    gs.chat_today += 1;
    stats.cooldown = 1.5;
    let base_rep = 0.8;
    let rep_bonus = if personality == NpcPersonality::Influential {
        3.0
    } else {
        0.0
    };
    rep.score = (rep.score + base_rep + rep_bonus).min(100.);
    let sp = if season.current == SeasonKind::Spring {
        " [Spring social!]"
    } else {
        ""
    };
    let tag = match personality {
        NpcPersonality::Cheerful => " [Cheerful]",
        NpcPersonality::Wise => " [Wise]",
        NpcPersonality::Influential => " [Influential]",
        NpcPersonality::Neutral => "",
    };
    let rr = if rep_friend_mult < 1.0 {
        " [Low rep -50% bond]"
    } else {
        ""
    };
    notif.push(
        format!(
            "Chat with {}! +{:.0} Mood  Friendship {}/5{}{}{}",
            npc_name, hap, lvl as u32, sp, tag, rr
        ),
        2.5,
    );
}

/// ActionKind::StudyCourse - library studying with random skill boost.
fn handle_study(
    gt: &mut GameTime,
    stats: &mut PlayerStats,
    skills: &mut Skills,
    gs: &mut GameState,
    notif: &mut Notification,
    season: &Season,
    weather: &WeatherKind,
) {
    if !stats.can_afford(30.) {
        notif.push("Need $30 to study!", 2.);
        return;
    }
    if stats.energy < 20. {
        notif.push("Too tired to study!", 2.);
        return;
    }
    stats.money -= 30.;
    stats.energy -= 20.;
    let season_bonus = if season.current == SeasonKind::Spring {
        0.25
    } else {
        0.
    };
    let rainy_study_mult = if *weather == WeatherKind::Rainy {
        1.25
    } else {
        1.0
    };
    let seed = (gt.day.wrapping_mul(1664525)).wrapping_add(gs.study_today * 999983) % 4;
    let study_gain = (0.5 + season_bonus) * stats.skill_gain_mult() * rainy_study_mult;
    let (boost_name, new_lvl) = match seed {
        0 => {
            skills.cooking = (skills.cooking + study_gain).min(5.);
            ("Cooking", skills.cooking)
        }
        1 => {
            skills.career = (skills.career + study_gain).min(5.);
            ("Career", skills.career)
        }
        2 => {
            skills.fitness = (skills.fitness + study_gain).min(5.);
            ("Fitness", skills.fitness)
        }
        _ => {
            skills.social = (skills.social + study_gain).min(5.);
            ("Social", skills.social)
        }
    };
    gs.study_today += 1;
    stats.cooldown = 3.;
    let sp = if season_bonus > 0. {
        " [Spring +0.25 bonus!]"
    } else {
        ""
    };
    let rsp = if rainy_study_mult > 1.0 {
        " [Rainy +25% XP]"
    } else {
        ""
    };
    notif.push(
        format!(
            "Studied! +{:.2} {} (now {:.1}){}{}",
            study_gain, boost_name, new_lvl, sp, rsp
        ),
        3.,
    );
}

pub fn handle_interaction(
    keys: Res<ButtonInput<KeyCode>>,
    nearby: Res<NearbyInteractable>,
    inter_q: Query<&Interactable>,
    npc_q: Query<(&Npc, &NpcId)>,
    mut player_q: Query<
        (
            &mut PlayerMovement,
            &mut VehicleState,
            &mut BankInput,
            &mut ActionPrompt,
        ),
        With<Player>,
    >,
    mut gt: ResMut<GameTime>,
    mut stats: ResMut<PlayerStats>,
    mut inv: ResMut<Inventory>,
    mut skills: ResMut<Skills>,
    mut friendship: ResMut<NpcFriendship>,
    mut gs: ResMut<GameState>,
    mut notif: ResMut<Notification>,
    mut streak: ResMut<WorkStreak>,
    mut housing: ResMut<HousingTier>,
    mut extras: InteractExtras,
    mut sfx: EventWriter<PlaySfx>,
) {
    let Ok((mut pm, mut vehicle_state, mut bank_input, mut action_prompt)) =
        player_q.get_single_mut()
    else {
        return;
    };
    let mut pe = keys.just_pressed(KeyCode::KeyE);
    let mut pg = keys.just_pressed(KeyCode::KeyG);
    let mut p1 = keys.just_pressed(KeyCode::Digit1);
    let mut p2 = keys.just_pressed(KeyCode::Digit2);
    let mut p3 = keys.just_pressed(KeyCode::Digit3);
    let mut p4 = keys.just_pressed(KeyCode::Digit4);
    let mut p5 = keys.just_pressed(KeyCode::Digit5);
    let mut p6 = keys.just_pressed(KeyCode::Digit6);
    let mut p7 = keys.just_pressed(KeyCode::Digit7);
    let mut p8 = keys.just_pressed(KeyCode::Digit8);
    let mut p9 = keys.just_pressed(KeyCode::Digit9);
    let mut forced_action: Option<ActionKind> = None;
    let mut forced_entity: Option<Entity> = None;

    if let Some((pending, target)) =
        handle_action_prompt_input(&keys, &mut action_prompt, &mut notif)
    {
        forced_entity = target;
        match pending {
            PendingAction::Action(action) => {
                pe = true;
                forced_action = Some(action);
            }
            PendingAction::Gift => {
                pg = true;
                forced_action = Some(ActionKind::Chat);
            }
            PendingAction::Bank(slot) => {
                forced_action = Some(ActionKind::Bank);
                match slot {
                    1 => p1 = true,
                    2 => p2 = true,
                    3 => p3 = true,
                    4 => p4 = true,
                    5 => p5 = true,
                    6 => p6 = true,
                    7 => p7 = true,
                    8 => p8 = true,
                    _ => p9 = true,
                }
            }
            PendingAction::Transport(slot) => {
                forced_action = Some(ActionKind::BuyTransport);
                match slot {
                    1 => p1 = true,
                    2 => p2 = true,
                    _ => p3 = true,
                }
            }
            PendingAction::Craft(slot) => {
                forced_action = Some(ActionKind::Craft);
                match slot {
                    1 => p1 = true,
                    2 => p2 = true,
                    _ => p3 = true,
                }
            }
            PendingAction::Festival(slot) => {
                forced_action = Some(ActionKind::Relax);
                match slot {
                    1 => p1 = true,
                    2 => p2 = true,
                    3 => p3 = true,
                    _ => p4 = true,
                }
            }
        }
    } else if action_prompt.active {
        return;
    }

    if (!pe && !pg && !p1 && !p2 && !p3 && !p4 && !p5 && !p6 && !p7 && !p8 && !p9)
        || stats.cooldown > 0.
    {
        return;
    }

    // Block all actions while hospitalised
    if extras.conds.hospitalized {
        let hrs = extras.conds.hospital_timer.ceil() as i32;
        notif.push(
            format!(
                "Hospitalised! Resting... ({} in-game hour{} left)",
                hrs,
                if hrs == 1 { "" } else { "s" }
            ),
            2.5,
        );
        return;
    }
    if bank_input.active {
        return;
    }
    let Some(entity) = forced_entity.or(nearby.entity) else {
        return;
    };
    let Ok(inter) = inter_q.get(entity) else {
        return;
    };

    if forced_action.is_none()
        && let Some(pending) = pending_action_from_input(
            inter,
            extras.festival.is_active(),
            pe,
            pg,
            p1,
            p2,
            p3,
            p4,
            p5,
            p6,
            p7,
            p8,
            p9,
        )
    {
        let prompt_subject = if matches!(
            &pending,
            PendingAction::Action(ActionKind::Chat) | PendingAction::Gift
        ) {
            npc_q
                .get(entity)
                .map(|(npc, _)| npc.name.as_str())
                .unwrap_or("friend")
        } else {
            extras.pet.name.as_str()
        };
        begin_action_prompt(
            &mut action_prompt,
            pending,
            entity,
            &gt,
            &skills,
            prompt_subject,
            &mut notif,
        );
        return;
    }

    let mut sfx_kind = SfxKind::Interact;

    // ── Bank key shortcuts ────────────────────────────────────────────────────
    if matches!(&inter.action, ActionKind::Bank)
        && handle_bank_keys(
            p1,
            p2,
            p3,
            p4,
            p5,
            p6,
            p7,
            p8,
            p9,
            &mut gt,
            &mut stats,
            &mut notif,
            &mut bank_input,
            &mut extras.invest,
            &mut extras.crisis,
        )
    {
        return;
    }

    // ── Transport key shortcuts ───────────────────────────────────────────────
    if matches!(&inter.action, ActionKind::BuyTransport)
        && handle_transport_keys(
            p1,
            p2,
            p3,
            &mut gt,
            &mut stats,
            &mut notif,
            &mut extras.transport,
        )
    {
        return;
    }

    // ── Craft key shortcuts ───────────────────────────────────────────────────
    if matches!(&inter.action, ActionKind::Craft) {
        if needs_home_access(&inter.action) && !housing.has_access() {
            let cost = housing.upgrade_cost().unwrap_or(90.);
            notif.push(
                format!(
                    "Locked out. Save ${:.0} and buy apartment access at the bank.",
                    cost
                ),
                3.5,
            );
            stats.cooldown = 0.5;
            return;
        }
        if handle_craft_keys(
            p1,
            p2,
            p3,
            &mut gt,
            &mut stats,
            &mut notif,
            &mut inv,
            &mut skills,
            &mut gs,
            &mut extras.quest_board,
        ) {
            return;
        }
    }

    if !pe && !pg && !(matches!(&inter.action, ActionKind::Relax) && (p1 || p2 || p3 || p4)) {
        return;
    }

    if needs_home_access(&inter.action) && !housing.has_access() {
        let cost = housing.upgrade_cost().unwrap_or(90.);
        notif.push(
            format!(
                "Locked out. Save ${:.0} and buy apartment access at the bank.",
                cost
            ),
            3.5,
        );
        stats.cooldown = 0.5;
        return;
    }

    // Appliance breakdown: home actions cost extra
    let repair_fee = extras.crisis.home_cost_extra();
    if repair_fee > 0. && needs_home_access(&inter.action) {
        if !stats.can_afford(repair_fee) {
            notif.push(
                format!(
                    "Appliance broken! Need ${:.0} repair fee for home actions.",
                    repair_fee
                ),
                3.,
            );
            return;
        }
        stats.money -= repair_fee;
    }

    if !matches!(&inter.action, ActionKind::Bank) {
        gt.advance_hours(action_time_hours(&inter.action));
    }

    match forced_action.unwrap_or_else(|| inter.action.clone()) {
        ActionKind::Sleep => {
            sfx_kind = SfxKind::Sleep;
            let base_gain = housing.sleep_energy(gt.is_night());
            let (stress_mult, stress_tag) = if stats.stress > 75. {
                (0.70_f32, " [Troubled -30%]")
            } else if stats.stress > 50. {
                (0.85_f32, " [Restless -15%]")
            } else {
                (1.0_f32, "")
            };
            let gain = base_gain * stress_mult;
            let health_gain = housing.night_health();
            stats.energy = (stats.energy + gain).min(stats.max_energy());
            stats.health = (stats.health + health_gain + 3.).min(100.);
            stats.stress = (stats.stress - 15.).max(0.);
            stats.sleep_debt = (stats.sleep_debt - 8.).max(0.);
            stats.cooldown = 3.;
            let tag = if gt.is_night() {
                "Night sleep"
            } else {
                "Daytime nap"
            };
            notif.push(
                format!(
                    "{} — +{:.0} Energy, -SleepDebt, -Stress{}",
                    tag, gain, stress_tag
                ),
                2.,
            );
        }
        ActionKind::Eat => {
            sfx_kind = SfxKind::Eat;
            let reduction = 40. * skills.cooking_bonus();
            let breakfast_bonus = if gt.is_breakfast() { 10. } else { 0. };
            if stats.meals > 0 {
                stats.meals -= 1;
            } else if stats.can_afford(10.) {
                stats.money -= 10.;
            } else {
                notif.push("No food or money!", 2.5);
                return;
            }
            let (meal_label, extra_health, extra_hap) = meal_tier(skills.cooking);
            stats.hunger = (stats.hunger - reduction).max(0.);
            stats.health = (stats.health + 1. + extra_health).min(100.);
            stats.happiness = (stats.happiness + extra_hap).min(100.);
            stats.energy = (stats.energy + breakfast_bonus).min(stats.max_energy());
            skills.cooking = (skills.cooking + 0.10 * stats.skill_gain_mult()).min(5.);
            gs.eat_today += 1;
            stats.cooldown = 2.;
            let bfast = if breakfast_bonus > 0. {
                " [Breakfast +10 Energy!]"
            } else {
                ""
            };
            notif.push(
                format!("{} — -{:.0} Hunger{}", meal_label, reduction, bfast),
                2.,
            );
        }
        ActionKind::Work => {
            sfx_kind = SfxKind::Work;
            handle_work(
                &mut gt,
                &mut stats,
                &mut skills,
                &mut gs,
                &mut notif,
                &mut streak,
                &housing,
                &extras.conds,
                &mut extras.transport,
                &mut extras.rep,
                &pm,
                &extras.settings,
                &extras.crisis,
            );
        }
        ActionKind::Freelance => {
            sfx_kind = SfxKind::Work;
            if stats.energy < 8. {
                notif.push("Too tired for freelance!", 2.);
                return;
            }
            let mood = Mood::from_happiness(stats.happiness);
            let base_pay = freelance_base_pay(skills.career);
            let earned = base_pay
                * mood.work_mult()
                * extras.rep.work_mult()
                * stats.loan_penalty()
                * extras.settings.difficulty.economy_mult();
            stats.money += earned;
            stats.energy = (stats.energy - 8.).max(0.);
            stats.stress = (stats.stress + 3.).min(100.);
            gs.work_today += 1;
            gs.money_earned_today += earned;
            streak.worked_today = true;
            extras.rep.score = (extras.rep.score + 0.3).min(100.);
            stats.cooldown = 2.;
            notif.push(format!("Freelanced from home. Earned ${:.0}.", earned), 2.);
        }
        ActionKind::Shop => {
            handle_shop(pe, p1, p2, p3, p4, &mut stats, &mut inv, &mut notif);
        }
        ActionKind::Relax => {
            handle_relax(
                p1,
                p2,
                p3,
                p4,
                &mut gt,
                &mut stats,
                &mut skills,
                &mut gs,
                &mut inv,
                &mut notif,
                &mut friendship,
                &mut extras.festival,
                &extras.weather,
                &extras.season,
                &mut extras.hobbies,
                &mut extras.rep,
                &extras.pet,
            );
        }
        ActionKind::Exercise => {
            if stats.energy < 20. {
                notif.push("Too tired to exercise!", 2.);
                return;
            }
            if extras.weather.is_stormy() {
                notif.push("Stormy! Exercise indoors instead.", 2.);
                return;
            }
            let season_mult = extras.season.current.social_mult();
            if *extras.weather == WeatherKind::Rainy {
                notif.push(
                    "Exercising in the rain! Consider going indoors instead.",
                    2.5,
                );
            }
            let fit_gain = (0.20 + skills.fitness * 0.02)
                * gt.exercise_mult()
                * season_mult
                * stats.skill_gain_mult();
            let exercise_cost = exercise_energy_cost(skills.fitness);
            stats.energy = (stats.energy - exercise_cost).max(0.);
            stats.health = (stats.health + 8. * skills.fitness_bonus()).min(100.);
            stats.hunger = (stats.hunger + 10.).min(100.);
            stats.happiness = (stats.happiness + 10.).min(100.);
            stats.stress = (stats.stress - 8.).max(0.);
            skills.fitness = (skills.fitness + fit_gain).min(5.);
            gs.exercise_today += 1;
            gs.outdoor_done_today = true;
            stats.cooldown = 3.;
            let am = if gt.exercise_mult() > 1.0 {
                " [Morning +25%]"
            } else {
                ""
            };
            let sp = if season_mult > 1.0 {
                " [Spring +25%]"
            } else {
                ""
            };
            notif.push(
                format!("Exercised! +Health, +Fitness {:.2}{}{}", fit_gain, am, sp),
                2.5,
            );
        }
        ActionKind::Meditate => {
            let hap_gain = 15. + skills.social * 2.;
            let fish_calm =
                extras.pet.has_pet && extras.pet.kind == PetKind::Fish && extras.pet.hunger < 80.;
            let (fish_hap, fish_stress, fish_tag) = if fish_calm {
                (8_f32, 8_f32, " [Fish +Calm]")
            } else {
                (0., 0., "")
            };
            stats.happiness = (stats.happiness + hap_gain + fish_hap).min(100.);
            stats.stress = (stats.stress - 25. - fish_stress).max(0.);
            stats.meditation_buff = 300.;
            stats.cooldown = 4.;
            notif.push(
                format!(
                    "Meditated. +{:.0} Happiness, -Stress, Zen buff 5h.{}",
                    hap_gain + fish_hap,
                    fish_tag
                ),
                3.,
            );
        }
        ActionKind::Shower => {
            stats.happiness = (stats.happiness + 12.).min(100.);
            stats.health = (stats.health + 2.).min(100.);
            stats.stress = (stats.stress - 5.).max(0.);
            stats.cooldown = 2.;
            notif.push("Showered! +12 Happiness, +Health, -Stress.", 2.);
        }
        ActionKind::Bank => {
            if let Some(cost) = housing.upgrade_cost() {
                if stats.savings >= cost {
                    if let Some(next) = housing.next() {
                        gt.advance_hours(0.25);
                        let had_access = housing.has_access();
                        stats.savings -= cost;
                        let label = next.label().to_string();
                        *housing = next;
                        let rent = housing.rent() as i32;
                        notif.push(
                            if had_access {
                            format!("Upgraded to {}! Savings -${:.0}", label, cost)
                        } else {
                            format!("Home secured! {} signed. Rent ${}/day — work daily to stay ahead.", label, rent)
                        },
                            7.,
                        );
                    }
                } else {
                    let next_label = housing
                        .next()
                        .map(|h| h.label().to_string())
                        .unwrap_or_else(|| "N/A".to_string());
                    let needed = (cost - stats.savings).max(0.);
                    notif.push(
                        if housing.has_access() {
                        format!(
                            "Bank: ${:.0} saved. [1]Dep [2]Wth [3]HalfDep [4]Loan [5]Repay [6]Invest(lo) [7]Invest(md) [8]CashOut [9]Insurance. Upgrade: ${:.0} for {}.",
                            stats.savings, cost, next_label
                        )
                    } else {
                        format!(
                            "Bank: ${:.0} saved — ${:.0} more for {}. [1]Deposit [2]Withdraw [3]HalfDep [4]Loan+$100 [5]Repay$50.",
                            stats.savings, needed, next_label
                        )
                    },
                        7.,
                    );
                }
            } else {
                notif.push(
                    format!("Bank: ${:.0} saved. Max housing! [1-8]", stats.savings),
                    4.,
                );
            }
            stats.cooldown = 0.5;
        }
        ActionKind::UseItem(kind) => {
            match kind {
                ItemKind::Coffee => {
                    if inv.coffee > 0 {
                        inv.coffee -= 1;
                        stats.energy = (stats.energy + 30.).min(stats.max_energy());
                        stats.cooldown = 1.;
                        notif.message = format!("Coffee! +30 Energy. ({}x left)", inv.coffee);
                    } else {
                        notif.message = "No coffee!".to_string();
                    }
                }
                ItemKind::Vitamins => {
                    if inv.vitamins > 0 {
                        inv.vitamins -= 1;
                        stats.health = (stats.health + 15.).min(100.);
                        stats.cooldown = 1.;
                        notif.message = format!("Vitamins! +15 Health. ({}x left)", inv.vitamins);
                    } else {
                        notif.message = "No vitamins!".to_string();
                    }
                }
                ItemKind::Books => {
                    if inv.books > 0 {
                        inv.books -= 1;
                        skills.career = (skills.career + 0.5).min(5.);
                        stats.cooldown = 2.;
                        notif.message = format!("Read! +0.5 Career XP. ({}x left)", inv.books);
                    } else {
                        notif.message = "No books!".to_string();
                    }
                }
                ItemKind::Smoothie => {
                    if inv.smoothie > 0 {
                        inv.smoothie -= 1;
                        stats.energy = (stats.energy + 40.).min(stats.max_energy());
                        stats.health = (stats.health + 10.).min(100.);
                        stats.cooldown = 1.;
                        notif.message =
                            format!("Smoothie! +40 Energy +10 Health. ({}x left)", inv.smoothie);
                    } else {
                        notif.message = "No smoothies! Craft one at home.".to_string();
                    }
                }
                ItemKind::Ingredient | ItemKind::GiftBox => {
                    notif.message = "This item can't be consumed directly.".to_string();
                }
            }
            notif.flush_message(2.);
        }
        ActionKind::Hobby(kind) => {
            if stats.energy < 10. {
                notif.push("Too tired for a hobby!", 2.);
                return;
            }
            let (skill_val, label) = match kind {
                HobbyKind::Painting => (&mut extras.hobbies.painting, "Painting"),
                HobbyKind::Gaming => (&mut extras.hobbies.gaming, "Gaming"),
                HobbyKind::Music => (&mut extras.hobbies.music, "Music"),
            };
            let rainy_mult = if *extras.weather == WeatherKind::Rainy {
                1.25
            } else {
                1.0
            };
            *skill_val = (*skill_val + 0.25 * stats.skill_gain_mult() * rainy_mult).min(5.);
            let lvl = *skill_val;
            let winter_bonus = extras.season.current.indoor_bonus();
            stats.happiness = (stats.happiness + 12. + winter_bonus).min(100.);
            stats.stress = (stats.stress - 8.).max(0.);
            stats.energy = (stats.energy - 10.).max(0.);
            gs.hobby_today += 1;
            stats.cooldown = 2.;
            let wb = if winter_bonus > 0. {
                " [Winter +cozy]"
            } else {
                ""
            };
            let rb = if rainy_mult > 1.0 {
                " [Rainy +25% XP]"
            } else {
                ""
            };
            notif.push(
                format!(
                    "{}! Skill: {:.2}/5. +Happiness, -Stress.{}{}",
                    label, lvl, wb, rb
                ),
                2.5,
            );
        }
        ActionKind::StudyCourse => {
            sfx_kind = SfxKind::Work;
            handle_study(
                &mut gt,
                &mut stats,
                &mut skills,
                &mut gs,
                &mut notif,
                &extras.season,
                &extras.weather,
            );
        }
        ActionKind::FeedPet => {
            if !extras.pet.has_pet {
                if stats.can_afford(50.) {
                    stats.money -= 50.;
                    extras.pet.has_pet = true;
                    extras.pet.fed_today = true;
                    extras.pet.hunger = 0.;
                    notif.message =
                        format!("Adopted {}! Feed daily for +happiness.", extras.pet.name);
                } else {
                    notif.message = "Need $50 to adopt a pet!".to_string();
                }
                notif.flush_message(3.);
                stats.cooldown = 0.5;
                return;
            }
            if !stats.can_afford(5.) {
                notif.push("Need $5 to feed your pet!", 2.);
                return;
            }
            stats.money -= 5.;
            extras.pet.hunger = 0.;
            extras.pet.fed_today = true;
            stats.happiness = (stats.happiness + 15.).min(100.);
            stats.stress = (stats.stress - 5.).max(0.);
            stats.cooldown = 1.;
            notif.push(
                format!("Fed {}! +15 Happiness, -Stress.", extras.pet.name),
                2.,
            );
        }
        ActionKind::ThrowParty => {
            if !stats.can_afford(40.) {
                notif.push("Need $40 to throw a party!", 2.);
                return;
            }
            if stats.energy < 20. {
                notif.push("Too tired to party!", 2.);
                return;
            }
            stats.money -= 40.;
            stats.energy -= 20.;
            stats.happiness = (stats.happiness + 30.).min(100.);
            stats.stress = (stats.stress - 10.).max(0.);
            extras.social_events.parties_thrown += 1;
            extras.social_events.party_today = true;
            extras.rep.score = (extras.rep.score + 5.).min(100.);
            stats.cooldown = 4.;
            notif.push(
                format!(
                    "Party thrown! +30 Happiness, +5 Rep. ({} total)",
                    extras.social_events.parties_thrown
                ),
                4.,
            );
        }
        ActionKind::BuyTransport => {
            let svc = if extras.transport.maintenance_due {
                " [NEEDS SERVICE!]"
            } else {
                ""
            };
            notif.push(
                format!(
                "Transport: {}{} [1] Bike $80sav  [2] Car $300sav  [3] Service $15 (every 5 uses)",
                extras.transport.kind.label(),
                svc,
            ),
                5.,
            );
            stats.cooldown = 0.5;
        }
        ActionKind::Chat => {
            let npc_data = npc_q.get(entity).ok();
            handle_chat(
                pe,
                pg,
                entity,
                &mut stats,
                &mut skills,
                &mut gs,
                &mut inv,
                &mut notif,
                &mut friendship,
                &mut extras.rep,
                &extras.season,
                npc_data,
            );
        }
        ActionKind::GymSession => {
            if !pe {
                return;
            }
            if stats.energy < 25. {
                notif.push("Too tired for the gym!", 2.);
                return;
            }
            if !stats.can_afford(5.) {
                notif.push("Need $5 for gym entry.", 2.);
                return;
            }
            let season_mult = extras.season.current.social_mult();
            let fit_gain = (0.25 + skills.fitness * 0.025)
                * gt.exercise_mult()
                * season_mult
                * stats.skill_gain_mult();
            stats.money -= 5.;
            stats.energy = (stats.energy - 25.).max(0.);
            stats.health = (stats.health + 12. * skills.fitness_bonus()).min(100.);
            stats.hunger = (stats.hunger + 12.).min(100.);
            stats.happiness = (stats.happiness + 12.).min(100.);
            stats.stress = (stats.stress - 12.).max(0.);
            skills.fitness = (skills.fitness + fit_gain).min(5.);
            gs.exercise_today += 1;
            stats.cooldown = 3.;
            let am = if gt.exercise_mult() > 1.0 {
                " [Morning +25%]"
            } else {
                ""
            };
            notif.push(
                format!("Gym session! +Health +Fit {:.2}{}  ($5 paid)", fit_gain, am),
                3.,
            );
            sfx_kind = SfxKind::Work;
        }
        ActionKind::Cafe => {
            if !pe {
                return;
            }
            if !stats.can_afford(12.) {
                notif.push("Need $12 for café order.", 2.);
                return;
            }
            stats.money -= 12.;
            stats.energy = (stats.energy + 25.).min(100.);
            stats.happiness = (stats.happiness + 12.).min(100.);
            stats.hunger = (stats.hunger - 25.).max(0.);
            extras.rep.score = (extras.rep.score + 2.).min(100.);
            stats.cooldown = 1.5;
            notif.push(
                "Café order! +25 Energy +12 Mood -25 Hunger  ($12 paid)",
                2.5,
            );
            sfx_kind = SfxKind::Eat;
        }
        ActionKind::Clinic => {
            if !pe {
                return;
            }
            if stats.health > 85. {
                notif.push("You're healthy — no clinic visit needed.", 2.);
                return;
            }
            if !stats.can_afford(40.) {
                notif.push("Need $40 for clinic visit.", 2.);
                return;
            }
            stats.money -= 40.;
            stats.health = (stats.health + 35.).min(100.);
            stats.stress = (stats.stress - 8.).max(0.);
            stats.cooldown = 2.;
            notif.push(
                format!("Clinic visit! +35 Health → {:.0}  ($40 paid)", stats.health),
                3.,
            );
        }
        ActionKind::EnterVehicle => {
            if extras.transport.kind != TransportKind::Car {
                notif.push("You don't own this car. Buy one at the Garage.", 2.5);
                return;
            }
            if vehicle_state.in_vehicle {
                vehicle_state.in_vehicle = false;
                pm.velocity = Vec2::ZERO;
                notif.message = "Exited car. Walk mode restored.".to_string();
            } else {
                vehicle_state.in_vehicle = true;
                notif.message = "Entered car! WASD to drive, E to exit.".to_string();
            }
            notif.flush_message(2.);
            stats.cooldown = 0.5;
        }
        ActionKind::AdoptPet(kind) => {
            if extras.pet.has_pet {
                notif.push(format!("You already have {}!", extras.pet.name), 2.);
                return;
            }
            if !stats.can_afford(300.) {
                notif.push("Need $300 to adopt a pet.", 2.);
                return;
            }
            stats.money -= 300.;
            extras.pet.has_pet = true;
            extras.pet.kind = kind;
            extras.pet.name = kind.name().to_string();
            extras.pet.hunger = 0.;
            extras.pet.fed_today = true;
            stats.happiness = (stats.happiness + 20.).min(100.);
            stats.cooldown = 1.;
            notif.push(
                format!("You adopted {}! Feed daily for bonuses.", extras.pet.name),
                4.,
            );
        }
        ActionKind::SleepRough => {
            stats.energy = (stats.energy + 25.).min(stats.max_energy());
            stats.health = (stats.health + 2.).min(100.);
            stats.stress = (stats.stress + 5.).min(100.);
            stats.happiness = (stats.happiness - 5.).max(0.);
            stats.sleep_debt = (stats.sleep_debt - 4.).max(0.);
            stats.cooldown = 3.;
            notif.push(
                "Slept rough - +25 Energy, slight +Stress, -Mood. Deposit at the Bank for a lease.",
                3.5,
            );
        }
        ActionKind::Craft => {
            notif.push(
                format!(
                    "Craft Station ({}x ingredients). [1] Cook Meal (2) [2] Gift Box (1+$5) [3] Smoothie (2)",
                    inv.ingredient
                ),
                5.,
            );
            stats.cooldown = 0.5;
        }
        ActionKind::RentUnit(unit_id) => {
            notif.push(
                format!(
                    "Apartment {}: rent here to gain home access and save on rent.",
                    unit_id
                ),
                4.,
            );
            stats.cooldown = 0.5;
        }
        ActionKind::GasUp => {
            if stats.can_afford(20.) {
                stats.money -= 20.;
                extras.transport.maintenance_due = false;
                notif.push("Gassed up! Vehicle range restored. ($20 paid)", 2.5);
            } else {
                notif.push("Need $20 to gas up.", 2.);
            }
            stats.cooldown = 0.5;
        }
        ActionKind::RepairVehicle => {
            if !extras.transport.kind.is_vehicle() {
                notif.push("No vehicle to repair!", 2.);
            } else if stats.can_afford(25.) {
                gt.advance_hours(1.0);
                stats.money -= 25.;
                extras.transport.maintenance_due = false;
                extras.transport.work_uses = 0;
                notif.push(
                    format!(
                        "Repaired {}! Pay bonus restored. ($25 paid)",
                        extras.transport.kind.label()
                    ),
                    3.,
                );
            } else {
                notif.push("Need $25 for a full vehicle repair.", 2.);
            }
            stats.cooldown = 0.5;
        }
        ActionKind::DentalVisit => {
            if stats.health > 90. {
                notif.push("Teeth look great — no visit needed.", 2.);
            } else if stats.can_afford(50.) {
                stats.money -= 50.;
                stats.health = (stats.health + 15.).min(100.);
                stats.stress = (stats.stress - 5.).max(0.);
                notif.push(
                    format!("Dental visit! +15 Health → {:.0}  ($50 paid)", stats.health),
                    3.,
                );
            } else {
                notif.push("Need $50 for a dental visit.", 2.);
            }
            stats.cooldown = 0.5;
        }
        ActionKind::EyeExam => {
            if stats.can_afford(35.) {
                stats.money -= 35.;
                stats.stress = (stats.stress - 8.).max(0.);
                stats.happiness = (stats.happiness + 8.).min(100.);
                notif.push("Eye exam done! -Stress +8 Mood  ($35 paid)", 2.5);
            } else {
                notif.push("Need $35 for an eye exam.", 2.);
            }
            stats.cooldown = 0.5;
        }
        ActionKind::ComputerLab => {
            handle_study(
                &mut gt,
                &mut stats,
                &mut skills,
                &mut gs,
                &mut notif,
                &extras.season,
                &extras.weather,
            );
        }
        ActionKind::PrintShop => {
            if stats.can_afford(5.) {
                stats.money -= 5.;
                notif.push("Printed documents. ($5 paid)", 2.);
            } else {
                notif.push("Need $5 to print.", 2.);
            }
            stats.cooldown = 0.25;
        }
    }
    sfx.send(PlaySfx(sfx_kind));
}

pub fn handle_bank_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_bank_q: Query<&mut BankInput, With<Player>>,
    mut stats: ResMut<PlayerStats>,
    mut notif: ResMut<Notification>,
    mut gt: ResMut<GameTime>,
    mut goal: ResMut<DailyGoal>,
) {
    let Ok(mut bank_input) = player_bank_q.get_single_mut() else {
        return;
    };
    if !bank_input.active {
        return;
    }

    let kind_str = if bank_input.kind == BankInputKind::Deposit {
        "Deposit"
    } else {
        "Withdraw"
    };
    notif.message = format!(
        "{} amount: ${}_ [Enter]=confirm [Esc]=cancel",
        kind_str, bank_input.buffer
    );
    notif.timer = 2.0;

    if keys.just_pressed(KeyCode::Escape) {
        bank_input.active = false;
        bank_input.buffer.clear();
        notif.push("Cancelled.", 1.5);
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        bank_input.buffer.pop();
        return;
    }

    let digit_keys = [
        (KeyCode::Digit0, '0'),
        (KeyCode::Digit1, '1'),
        (KeyCode::Digit2, '2'),
        (KeyCode::Digit3, '3'),
        (KeyCode::Digit4, '4'),
        (KeyCode::Digit5, '5'),
        (KeyCode::Digit6, '6'),
        (KeyCode::Digit7, '7'),
        (KeyCode::Digit8, '8'),
        (KeyCode::Digit9, '9'),
    ];
    for (kc, ch) in digit_keys {
        if keys.just_pressed(kc) && bank_input.buffer.len() < 7 {
            bank_input.buffer.push(ch);
        }
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        let amount: f32 = bank_input.buffer.parse().unwrap_or(0.);
        bank_input.active = false;
        bank_input.buffer.clear();
        if amount <= 0. {
            notif.push("Invalid amount.", 2.);
            return;
        }
        match bank_input.kind {
            BankInputKind::Deposit => {
                if let Some((new_money, new_savings)) =
                    try_deposit(stats.money, stats.savings, amount)
                {
                    stats.money = new_money;
                    stats.savings = new_savings;
                    stats.stress = (stats.stress - 3.).max(0.);
                    gt.advance_hours(0.25);
                    if matches!(&goal.kind, GoalKind::SaveMoney) && !goal.completed {
                        goal.progress = stats.savings;
                        if goal.progress >= goal.target {
                            goal.completed = true;
                            stats.money += goal.reward_money;
                            stats.happiness = (stats.happiness + goal.reward_happiness).min(100.);
                            notif.push(format!("Deposited ${:.0}! +${:.0} +{}hap — Press [E] at bank to sign your lease.", amount, goal.reward_money, goal.reward_happiness as i32), 7.);
                            return;
                        }
                    }
                    notif.push(
                        format!("Deposited ${:.0}. Savings: ${:.0}", amount, stats.savings),
                        3.,
                    );
                } else {
                    notif.push(format!("Not enough cash! (Have ${:.0})", stats.money), 2.);
                }
            }
            BankInputKind::Withdraw => {
                if let Some((new_savings, new_money)) =
                    try_withdraw(stats.savings, stats.money, amount)
                {
                    stats.savings = new_savings;
                    stats.money = new_money;
                    gt.advance_hours(0.25);
                    notif.push(
                        format!("Withdrew ${:.0}. Savings: ${:.0}", amount, stats.savings),
                        3.,
                    );
                } else {
                    notif.push(
                        format!("Not enough savings! (Have ${:.0})", stats.savings),
                        2.,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── health_work_mult ──────────────────────────────────────────────────────

    #[test]
    fn health_mult_full_health_is_one() {
        assert!((health_work_mult(100.) - 1.0).abs() < f32::EPSILON);
        assert!((health_work_mult(50.) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn health_mult_unwell_range() {
        assert!((health_work_mult(49.) - 0.90).abs() < f32::EPSILON);
        assert!((health_work_mult(25.) - 0.90).abs() < f32::EPSILON);
    }

    #[test]
    fn health_mult_poor_health() {
        assert!((health_work_mult(24.) - 0.75).abs() < f32::EPSILON);
        assert!((health_work_mult(0.) - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn health_mult_boundary_at_25() {
        assert!((health_work_mult(25.) - 0.90).abs() < f32::EPSILON);
        assert!((health_work_mult(24.9) - 0.75).abs() < f32::EPSILON);
    }

    // ── freelance_base_pay ────────────────────────────────────────────────────

    #[test]
    fn freelance_pay_junior() {
        assert!((freelance_base_pay(0.0) - 15.).abs() < f32::EPSILON);
        assert!((freelance_base_pay(2.49) - 15.).abs() < f32::EPSILON);
    }

    #[test]
    fn freelance_pay_senior() {
        assert!((freelance_base_pay(2.5) - 22.).abs() < f32::EPSILON);
        assert!((freelance_base_pay(4.9) - 22.).abs() < f32::EPSILON);
    }

    #[test]
    fn freelance_pay_mastery() {
        assert!((freelance_base_pay(5.0) - 35.).abs() < f32::EPSILON);
    }

    // ── meal_tier ─────────────────────────────────────────────────────────────

    #[test]
    fn meal_tier_labels() {
        assert_eq!(meal_tier(0.0).0, "Basic meal");
        assert_eq!(meal_tier(2.0).0, "Good meal");
        assert_eq!(meal_tier(4.0).0, "Gourmet meal");
        assert_eq!(meal_tier(5.0).0, "Master Chef meal");
    }

    #[test]
    fn meal_tier_extra_health_values() {
        assert!((meal_tier(5.0).1 - 10.).abs() < f32::EPSILON);
        assert!((meal_tier(4.0).1 - 6.).abs() < f32::EPSILON);
        assert!((meal_tier(2.0).1 - 3.).abs() < f32::EPSILON);
        assert!((meal_tier(0.0).1 - 0.).abs() < f32::EPSILON);
    }

    #[test]
    fn meal_tier_boundary_at_2_and_4() {
        assert_eq!(meal_tier(1.99).0, "Basic meal");
        assert_eq!(meal_tier(2.0).0, "Good meal");
        assert_eq!(meal_tier(3.99).0, "Good meal");
        assert_eq!(meal_tier(4.0).0, "Gourmet meal");
        assert_eq!(meal_tier(4.99).0, "Gourmet meal");
        assert_eq!(meal_tier(5.0).0, "Master Chef meal");
    }

    // ── exercise_energy_cost ──────────────────────────────────────────────────

    #[test]
    fn exercise_cost_normal() {
        assert!((exercise_energy_cost(0.0) - 20.).abs() < f32::EPSILON);
        assert!((exercise_energy_cost(4.99) - 20.).abs() < f32::EPSILON);
    }

    #[test]
    fn exercise_cost_mastery() {
        assert!((exercise_energy_cost(5.0) - 10.).abs() < f32::EPSILON);
    }

    // ── try_deposit ───────────────────────────────────────────────────────────

    #[test]
    fn deposit_succeeds_when_funds_sufficient() {
        let result = try_deposit(100., 50., 40.);
        assert!(result.is_some());
        let (new_money, new_savings) = result.unwrap();
        assert!((new_money - 60.).abs() < f32::EPSILON);
        assert!((new_savings - 90.).abs() < f32::EPSILON);
    }

    #[test]
    fn deposit_exact_amount_leaves_zero_cash() {
        let (new_money, new_savings) = try_deposit(50., 0., 50.).unwrap();
        assert!((new_money - 0.).abs() < f32::EPSILON);
        assert!((new_savings - 50.).abs() < f32::EPSILON);
    }

    #[test]
    fn deposit_fails_when_insufficient_cash() {
        assert!(try_deposit(30., 100., 50.).is_none());
    }

    #[test]
    fn deposit_fails_on_zero_amount() {
        assert!(try_deposit(100., 0., 0.).is_none());
    }

    #[test]
    fn deposit_fails_on_negative_amount() {
        assert!(try_deposit(100., 0., -10.).is_none());
    }

    // ── try_withdraw ──────────────────────────────────────────────────────────

    #[test]
    fn withdraw_succeeds_when_savings_sufficient() {
        let (new_savings, new_money) = try_withdraw(200., 10., 75.).unwrap();
        assert!((new_savings - 125.).abs() < f32::EPSILON);
        assert!((new_money - 85.).abs() < f32::EPSILON);
    }

    #[test]
    fn withdraw_exact_savings_leaves_zero() {
        let (new_savings, new_money) = try_withdraw(100., 0., 100.).unwrap();
        assert!((new_savings - 0.).abs() < f32::EPSILON);
        assert!((new_money - 100.).abs() < f32::EPSILON);
    }

    #[test]
    fn withdraw_fails_when_insufficient_savings() {
        assert!(try_withdraw(50., 100., 75.).is_none());
    }

    #[test]
    fn withdraw_fails_on_zero_amount() {
        assert!(try_withdraw(100., 50., 0.).is_none());
    }

    #[test]
    fn prompt_retries_drop_as_career_rises() {
        assert_eq!(action_prompt_retries(0.0), 4);
        assert_eq!(action_prompt_retries(2.5), 3);
        assert_eq!(action_prompt_retries(5.0), 2);
    }

    #[test]
    fn work_prompt_requires_three_words() {
        let challenge =
            build_prompt_challenge(&PendingAction::Action(ActionKind::Work), 0, "buddy");
        assert_eq!(challenge.expected.split_whitespace().count(), 3);
        assert!(challenge.label.contains("Work"));
    }

    #[test]
    fn display_text_shows_expected_phrase() {
        let prompt = ActionPrompt {
            active: true,
            buffer: "desk".to_string(),
            label: "Work".to_string(),
            instruction: "type the office words in order".to_string(),
            expected: "desk report email".to_string(),
            retries_left: 4,
            pending: None,
            target: None,
        };

        let display = prompt.display_text();
        assert!(display.contains("desk report email"));
        assert!(display.contains("desk_"));
    }

    #[test]
    fn eat_prompt_starts_with_eat_keyword() {
        let challenge = build_prompt_challenge(&PendingAction::Action(ActionKind::Eat), 7, "buddy");
        assert!(challenge.expected.starts_with("eat "));
        assert!(challenge.label.contains("Eat"));
    }

    #[test]
    fn chat_prompt_uses_subject_name() {
        let challenge = build_prompt_challenge(&PendingAction::Action(ActionKind::Chat), 3, "alex");
        assert!(challenge.expected.contains("alex"));
    }
}
