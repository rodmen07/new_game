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
        ActionKind::Hangout => 1.0,
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

// ── Word banks — one random word is picked per challenge ──────────────────────
const WORK_WORDS: &[&str] = &[
    "deadline",
    "pivot",
    "synergy",
    "workflow",
    "sprint",
    "standup",
    "backlog",
    "invoice",
    "quarterly",
    "metric",
    "onboard",
    "milestone",
    "overtime",
    "feedback",
    "proposal",
    "agenda",
    "pipeline",
    "deploy",
    "dashboard",
    "offsite",
];
const FREELANCE_WORDS: &[&str] = &[
    "invoice", "client", "remote", "contract", "revise", "proposal", "deadline", "pitch",
    "project", "retainer", "markup", "estimate", "draft", "scope", "deliver", "iterate",
    "approval", "mockup", "handoff", "billing",
];
const EAT_WORDS: &[&str] = &[
    "bowl", "sandwich", "noodles", "salad", "stew", "toast", "wrap", "burrito", "ramen", "curry",
    "falafel", "pasta", "taco", "grains", "broth",
];
const SLEEP_WORDS: &[&str] = &[
    "slumber",
    "doze",
    "nap",
    "recharge",
    "rest",
    "snooze",
    "unwind",
    "hibernate",
    "drift",
    "recover",
    "pillow",
    "blanket",
    "drowsy",
    "twilight",
    "dream",
];
const SHOP_WORDS: &[&str] = &[
    "groceries",
    "staples",
    "rations",
    "supplies",
    "provisions",
    "checkout",
    "essentials",
    "stock",
    "cart",
    "purchase",
    "restock",
    "budget",
    "basket",
    "aisle",
    "coupon",
];
const RELAX_WORDS: &[&str] = &[
    "park", "bench", "garden", "shade", "breeze", "sunset", "wander", "lounge", "stroll", "unwind",
    "meadow", "birdsong", "hammock", "leisure", "picnic",
];
const SHOWER_WORDS: &[&str] = &[
    "rinse",
    "scrub",
    "lather",
    "freshen",
    "cleanse",
    "refresh",
    "steam",
    "towel",
    "exfoliate",
    "hygiene",
    "soapy",
    "warm",
    "drench",
    "nozzle",
    "squeaky",
];
const EXERCISE_WORDS: &[&str] = &[
    "sprint", "squat", "burpee", "pushup", "plank", "lunge", "crunch", "pullup", "deadlift", "jog",
    "interval", "tempo", "stride", "circuit", "rally",
];
const MEDITATE_WORDS: &[&str] = &[
    "breathe",
    "focus",
    "clarity",
    "stillness",
    "balance",
    "center",
    "mindful",
    "inhale",
    "exhale",
    "serenity",
    "grounded",
    "aware",
    "presence",
    "tranquil",
    "quiet",
];
const STUDY_WORDS: &[&str] = &[
    "chapter",
    "lecture",
    "notes",
    "review",
    "research",
    "quiz",
    "outline",
    "flashcard",
    "summary",
    "concept",
    "thesis",
    "annotate",
    "memorize",
    "syllabus",
    "focus",
];
const GYM_WORDS: &[&str] = &[
    "bench", "barbell", "squat", "deadlift", "cable", "dumbbell", "rack", "spotter", "shrug",
    "curl", "incline", "crunch", "lateral", "cardio", "reps",
];
const CAFE_WORDS: &[&str] = &[
    "latte",
    "espresso",
    "mocha",
    "cortado",
    "cappuccino",
    "drip",
    "matcha",
    "oat",
    "americano",
    "ristretto",
    "lungo",
    "macchiato",
    "chai",
    "froth",
    "coldbrew",
];
const CLINIC_WORDS: &[&str] = &[
    "checkup",
    "vitals",
    "bloodwork",
    "prescription",
    "diagnosis",
    "triage",
    "consult",
    "referral",
    "dosage",
    "followup",
    "symptom",
    "physician",
    "wellness",
    "recovery",
    "screening",
];
const VEHICLE_WORDS: &[&str] = &[
    "ignition",
    "clutch",
    "throttle",
    "reverse",
    "neutral",
    "cruise",
    "gear",
    "accelerate",
    "signal",
    "merge",
    "navigate",
    "steer",
    "parallel",
    "overtake",
    "brake",
];
const PARTY_WORDS: &[&str] = &[
    "gather",
    "toast",
    "celebrate",
    "invite",
    "mingle",
    "cheer",
    "festive",
    "decorate",
    "groove",
    "confetti",
    "clink",
    "revel",
    "playlist",
    "welcome",
    "candles",
];
const GARAGE_WORDS: &[&str] = &[
    "wrench",
    "torque",
    "socket",
    "lube",
    "patch",
    "inflate",
    "tighten",
    "flush",
    "grease",
    "align",
    "calibrate",
    "overhaul",
    "seal",
    "replace",
    "inspect",
];
const PRINT_WORDS: &[&str] = &[
    "collate",
    "format",
    "export",
    "submit",
    "staple",
    "laminate",
    "photocopy",
    "scan",
    "layout",
    "margins",
    "duplex",
    "toner",
    "spool",
    "queue",
    "printout",
];
const COMPUTER_WORDS: &[&str] = &[
    "compile", "debug", "deploy", "commit", "merge", "refactor", "lint", "optimize", "profile",
    "review", "branch", "clone", "iterate", "document", "build",
];
const DENTAL_WORDS: &[&str] = &[
    "floss",
    "rinse",
    "brace",
    "crown",
    "polish",
    "cavity",
    "enamel",
    "retainer",
    "bridge",
    "scale",
    "plaque",
    "fluoride",
    "whitening",
    "molar",
    "gum",
];
const EYE_WORDS: &[&str] = &[
    "focus",
    "dilate",
    "contrast",
    "distance",
    "strain",
    "clarity",
    "glare",
    "chart",
    "lens",
    "pupil",
    "retina",
    "blink",
    "optometry",
    "acuity",
    "peripheral",
];
const RENT_WORDS: &[&str] = &[
    "lease", "sign", "commit", "settle", "reside", "occupy", "tenant", "secure", "contract",
    "deposit", "landlord", "furnish", "movein", "clauses", "renew",
];
const GAS_WORDS: &[&str] = &[
    "fuel", "refill", "tank", "pump", "gasoline", "diesel", "unleaded", "nozzle", "octane",
    "premium", "regular", "station", "receipt", "gallons", "liters",
];
const REPAIR_WORDS: &[&str] = &[
    "weld",
    "patch",
    "bolt",
    "tighten",
    "replace",
    "overhaul",
    "seal",
    "solder",
    "rebuild",
    "service",
    "grease",
    "swap",
    "calibrate",
    "inspect",
    "restore",
];
const ITEM_COFFEE_WORDS: &[&str] = &[
    "espresso",
    "latte",
    "brew",
    "arabica",
    "roast",
    "java",
    "buzz",
    "caffeine",
    "drip",
    "aroma",
    "percolate",
    "grounds",
    "crema",
    "filter",
    "mug",
];
const ITEM_VITAMINS_WORDS: &[&str] = &[
    "vitamin",
    "supplement",
    "capsule",
    "zinc",
    "omega",
    "boost",
    "daily",
    "nutrient",
    "mineral",
    "biotin",
    "probiotic",
    "collagen",
    "immunity",
    "antioxidant",
    "dose",
];
const ITEM_BOOKS_WORDS: &[&str] = &[
    "chapter",
    "novel",
    "passage",
    "volume",
    "excerpt",
    "prose",
    "absorb",
    "bookmark",
    "narrative",
    "plot",
    "insight",
    "words",
    "knowledge",
    "reading",
    "index",
];
const ITEM_INGREDIENT_WORDS: &[&str] = &[
    "carrot",
    "basil",
    "ginger",
    "garlic",
    "thyme",
    "cumin",
    "pepper",
    "zest",
    "saffron",
    "oregano",
    "paprika",
    "turmeric",
    "coriander",
    "cinnamon",
    "chili",
];
const ITEM_GIFTBOX_WORDS: &[&str] = &[
    "ribbon",
    "wrap",
    "package",
    "bundle",
    "seal",
    "bow",
    "surprise",
    "keepsake",
    "token",
    "gesture",
    "giving",
    "present",
    "cherish",
    "celebrate",
    "memories",
];
const ITEM_SMOOTHIE_WORDS: &[&str] = &[
    "berry",
    "blend",
    "mango",
    "tropical",
    "citrus",
    "kale",
    "protein",
    "frothy",
    "spinach",
    "avocado",
    "coconut",
    "ginger",
    "peach",
    "pineapple",
    "flax",
];
const PAINTING_WORDS: &[&str] = &[
    "canvas",
    "stroke",
    "palette",
    "acrylic",
    "texture",
    "blend",
    "layer",
    "hue",
    "sketch",
    "detail",
    "pigment",
    "easel",
    "chiaroscuro",
    "impasto",
    "glaze",
];
const GAMING_WORDS: &[&str] = &[
    "respawn", "loot", "quest", "dungeon", "boss", "combo", "stealth", "level", "vault", "dodge",
    "craft", "rally", "trigger", "snipe", "grind",
];
const MUSIC_WORDS: &[&str] = &[
    "chord", "rhythm", "melody", "tempo", "harmony", "riff", "verse", "bridge", "scale", "groove",
    "strum", "pitch", "dynamics", "accent", "cadence",
];
const BANK_DEPOSIT_WORDS: &[&str] = &[
    "deposit",
    "save",
    "stash",
    "secure",
    "lodge",
    "store",
    "fund",
    "contribute",
];
const BANK_WITHDRAW_WORDS: &[&str] = &[
    "withdraw", "collect", "redeem", "retrieve", "cash", "pocket", "access", "release",
];
const BANK_INVEST_WORDS: &[&str] = &[
    "invest",
    "growth",
    "yield",
    "compound",
    "dividend",
    "stake",
    "hedge",
    "diversify",
];
const BANK_LOAN_WORDS: &[&str] = &[
    "borrow",
    "loan",
    "credit",
    "advance",
    "finance",
    "collateral",
];
const BANK_HALF_WORDS: &[&str] = &["partial", "split", "half", "divide", "portion", "share"];
const BANK_REPAY_WORDS: &[&str] = &["repay", "settle", "clear", "return", "payoff", "discharge"];
const BANK_CASHOUT_WORDS: &[&str] = &[
    "cashout",
    "liquidate",
    "redeem",
    "convert",
    "exit",
    "encash",
];
const BANK_INSURE_WORDS: &[&str] = &[
    "insure",
    "coverage",
    "protect",
    "policy",
    "safeguard",
    "shield",
];
const GIFT_WORDS: &[&str] = &[
    "present", "token", "keepsake", "souvenir", "surprise", "offering", "gesture", "bouquet",
    "charm", "trinket", "memento", "cherish",
];
const BIKE_WORDS: &[&str] = &[
    "pedal",
    "chain",
    "saddle",
    "commute",
    "gear",
    "spoke",
    "handlebar",
    "brake",
    "cycle",
    "sprint",
];
const CAR_WORDS: &[&str] = &[
    "ignition",
    "clutch",
    "navigate",
    "accelerate",
    "signal",
    "reverse",
    "cruise",
    "steer",
    "parallel",
    "drive",
];
const SERVICE_WORDS: &[&str] = &[
    "service", "inspect", "lube", "filter", "align", "tune", "replace", "torque", "flush",
    "overhaul",
];
const COOK_WORDS: &[&str] = &[
    "stir",
    "chop",
    "saute",
    "simmer",
    "blanch",
    "season",
    "dice",
    "roast",
    "fold",
    "reduce",
    "whisk",
    "baste",
    "caramelize",
    "puree",
    "flambe",
];
const SMOOTHIE_WORDS: &[&str] = &[
    "blend", "puree", "pour", "chill", "freeze", "whisk", "crush", "shake", "swirl", "mix",
    "liquefy", "froth", "churn", "emulsify", "strain",
];
const FESTIVAL_WORDS: &[&str] = &[
    "cheer",
    "dance",
    "mingle",
    "feast",
    "celebrate",
    "perform",
    "exhibit",
    "gather",
    "toast",
    "revel",
    "parade",
    "sparkle",
    "jubilee",
    "carnival",
    "exuberant",
];
const HANGOUT_WORDS: &[&str] = &[
    "coffee", "wander", "park", "chill", "stroll", "laugh", "picnic", "explore", "catch-up",
    "banter", "unwind", "hang", "vibe", "lounge", "chat",
];

fn pick_word(pool: &'static [&'static str], seed: u32, offset: u32) -> &'static str {
    pool[(seed.wrapping_add(offset * 11) as usize) % pool.len()]
}

fn word_challenge(
    label: &str,
    instruction: &str,
    words: &'static [&'static str],
    seed: u32,
) -> PromptChallenge {
    PromptChallenge {
        label: label.to_string(),
        instruction: instruction.to_string(),
        expected: pick_word(words, seed, 0).to_string(),
    }
}

fn build_prompt_challenge(
    pending: &PendingAction,
    seed: u32,
    subject_name: &str,
) -> PromptChallenge {
    let subject = if subject_name.trim().is_empty() {
        "friend".to_string()
    } else {
        subject_name.trim().to_lowercase()
    };
    match pending {
        PendingAction::Action(kind) => action_challenge(kind, seed, &subject),
        PendingAction::Gift => social_challenge(&subject, seed),
        PendingAction::Bank(slot) => finance_challenge(*slot, seed),
        PendingAction::Transport(slot) => transport_challenge(*slot, seed),
        PendingAction::Craft(slot) => craft_challenge(*slot, seed),
        PendingAction::Festival(slot) => festival_challenge(*slot, seed),
    }
}

fn action_challenge(kind: &ActionKind, seed: u32, subject: &str) -> PromptChallenge {
    match kind {
        ActionKind::Chat => PromptChallenge {
            label: "Chat".to_string(),
            instruction: format!("type to greet {}", subject),
            expected: subject.to_string(),
        },
        ActionKind::FeedPet => PromptChallenge {
            label: "Feed Pet".to_string(),
            instruction: format!("type to feed {}", subject),
            expected: subject.to_string(),
        },
        ActionKind::AdoptPet(k) => PromptChallenge {
            label: "Adopt Pet".to_string(),
            instruction: "type to adopt".to_string(),
            expected: k.label().to_lowercase(),
        },
        ActionKind::RentUnit(_) => word_challenge("Rent", "type to rent", RENT_WORDS, seed),
        ActionKind::UseItem(item) => item_challenge(item, seed),
        ActionKind::Hobby(hobby) => hobby_challenge(hobby, seed),
        ActionKind::Work => word_challenge("Work", "type to work", WORK_WORDS, seed),
        ActionKind::Freelance => {
            word_challenge("Freelance", "type to freelance", FREELANCE_WORDS, seed)
        }
        ActionKind::Eat => word_challenge("Eat", "type to eat", EAT_WORDS, seed),
        ActionKind::Sleep => word_challenge("Sleep", "type to sleep", SLEEP_WORDS, seed),
        ActionKind::SleepRough => word_challenge("Shelter", "type to shelter", SLEEP_WORDS, seed),
        ActionKind::Shop => word_challenge("Shop", "type to shop", SHOP_WORDS, seed),
        ActionKind::Relax => word_challenge("Relax", "type to relax", RELAX_WORDS, seed),
        ActionKind::Shower => word_challenge("Shower", "type to shower", SHOWER_WORDS, seed),
        ActionKind::Exercise => {
            word_challenge("Exercise", "type to exercise", EXERCISE_WORDS, seed)
        }
        ActionKind::Meditate => {
            word_challenge("Meditate", "type to meditate", MEDITATE_WORDS, seed)
        }
        ActionKind::Bank => word_challenge(
            "Bank",
            "type to open bank",
            &["vault", "access", "enter", "open"],
            seed,
        ),
        ActionKind::StudyCourse => word_challenge("Study", "type to study", STUDY_WORDS, seed),
        ActionKind::ThrowParty => word_challenge("Party", "type to host", PARTY_WORDS, seed),
        ActionKind::BuyTransport => {
            word_challenge("Transport", "type for garage", GARAGE_WORDS, seed)
        }
        ActionKind::GymSession => word_challenge("Gym", "type to train", GYM_WORDS, seed),
        ActionKind::Cafe => word_challenge("Cafe", "type to order", CAFE_WORDS, seed),
        ActionKind::Clinic => word_challenge("Clinic", "type for checkup", CLINIC_WORDS, seed),
        ActionKind::EnterVehicle => word_challenge("Vehicle", "type to drive", VEHICLE_WORDS, seed),
        ActionKind::Craft => word_challenge("Craft", "type to craft", COOK_WORDS, seed),
        ActionKind::GasUp => word_challenge("Gas Up", "type to refuel", GAS_WORDS, seed),
        ActionKind::RepairVehicle => word_challenge("Repair", "type to repair", REPAIR_WORDS, seed),
        ActionKind::DentalVisit => word_challenge("Dental", "type for dental", DENTAL_WORDS, seed),
        ActionKind::EyeExam => word_challenge("Eye Exam", "type for eye exam", EYE_WORDS, seed),
        ActionKind::ComputerLab => {
            word_challenge("Computer Lab", "type to log in", COMPUTER_WORDS, seed)
        }
        ActionKind::PrintShop => word_challenge("Print", "type to print", PRINT_WORDS, seed),
        ActionKind::Hangout => PromptChallenge {
            label: "Hangout".to_string(),
            instruction: format!("type to hang out with {}", subject),
            expected: pick_word(HANGOUT_WORDS, seed, 0).to_string(),
        },
    }
}

fn item_challenge(item: &ItemKind, seed: u32) -> PromptChallenge {
    match item {
        ItemKind::Coffee => {
            word_challenge("Coffee", "type to drink coffee", ITEM_COFFEE_WORDS, seed)
        }
        ItemKind::Vitamins => word_challenge(
            "Vitamins",
            "type to take vitamins",
            ITEM_VITAMINS_WORDS,
            seed,
        ),
        ItemKind::Books => word_challenge("Books", "type to read", ITEM_BOOKS_WORDS, seed),
        ItemKind::Ingredient => word_challenge(
            "Ingredient",
            "type to use ingredient",
            ITEM_INGREDIENT_WORDS,
            seed,
        ),
        ItemKind::GiftBox => {
            word_challenge("Gift Box", "type to open gift", ITEM_GIFTBOX_WORDS, seed)
        }
        ItemKind::Smoothie => word_challenge(
            "Smoothie",
            "type to drink smoothie",
            ITEM_SMOOTHIE_WORDS,
            seed,
        ),
    }
}

fn hobby_challenge(hobby: &HobbyKind, seed: u32) -> PromptChallenge {
    match hobby {
        HobbyKind::Painting => word_challenge("Painting", "type to paint", PAINTING_WORDS, seed),
        HobbyKind::Gaming => word_challenge("Gaming", "type to game", GAMING_WORDS, seed),
        HobbyKind::Music => word_challenge("Music", "type to play music", MUSIC_WORDS, seed),
    }
}

fn social_challenge(subject: &str, seed: u32) -> PromptChallenge {
    PromptChallenge {
        label: "Gift".to_string(),
        instruction: format!("type to give a gift to {}", subject),
        expected: pick_word(GIFT_WORDS, seed, 0).to_string(),
    }
}

fn finance_challenge(slot: u8, seed: u32) -> PromptChallenge {
    let (label, words): (&str, &[&str]) = match slot {
        1 => ("Deposit", BANK_DEPOSIT_WORDS),
        2 => ("Withdraw", BANK_WITHDRAW_WORDS),
        3 => ("Half Deposit", BANK_HALF_WORDS),
        4 => ("Loan", BANK_LOAN_WORDS),
        5 => ("Repay", BANK_REPAY_WORDS),
        6 => ("Invest", BANK_INVEST_WORDS),
        7 => ("Medium Invest", BANK_INVEST_WORDS),
        8 => ("Cash Out", BANK_CASHOUT_WORDS),
        _ => ("Insurance", BANK_INSURE_WORDS),
    };
    PromptChallenge {
        label: label.to_string(),
        instruction: "type to confirm banking".to_string(),
        expected: pick_word(words, seed, slot as u32).to_string(),
    }
}

fn transport_challenge(slot: u8, seed: u32) -> PromptChallenge {
    let (label, words): (&str, &[&str]) = match slot {
        1 => ("Buy Bike", BIKE_WORDS),
        2 => ("Buy Car", CAR_WORDS),
        _ => ("Service", SERVICE_WORDS),
    };
    PromptChallenge {
        label: label.to_string(),
        instruction: "type to confirm transport".to_string(),
        expected: pick_word(words, seed, slot as u32).to_string(),
    }
}

fn craft_challenge(slot: u8, seed: u32) -> PromptChallenge {
    let (label, words): (&str, &[&str]) = match slot {
        1 => ("Cook", COOK_WORDS),
        2 => ("Gift Box", ITEM_GIFTBOX_WORDS),
        _ => ("Smoothie", SMOOTHIE_WORDS),
    };
    PromptChallenge {
        label: label.to_string(),
        instruction: "type to craft".to_string(),
        expected: pick_word(words, seed, slot as u32).to_string(),
    }
}

fn festival_challenge(slot: u8, seed: u32) -> PromptChallenge {
    PromptChallenge {
        label: "Festival".to_string(),
        instruction: "type to join the festival".to_string(),
        expected: pick_word(FESTIVAL_WORDS, seed, slot as u32).to_string(),
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
    notif.push(format!("{} challenge!", prompt.label), 1.5);
}

fn handle_action_prompt_input(
    keys: &ButtonInput<KeyCode>,
    prompt: &mut ActionPrompt,
    notif: &mut Notification,
    sfx: &mut EventWriter<PlaySfx>,
) -> Option<(PendingAction, Option<Entity>)> {
    if !prompt.active {
        return None;
    }

    if keys.just_pressed(KeyCode::Escape) {
        prompt.clear();
        notif.push("Typing challenge cancelled.", 1.5);
        return None;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        prompt.buffer.pop();
        return None;
    }

    let len_before = prompt.buffer.len();
    collect_prompt_text(keys, &mut prompt.buffer);
    if prompt.buffer.len() > len_before {
        sfx.send(PlaySfx(SfxKind::KeyPress));
    }

    // Auto-confirm when the typed buffer matches the expected single word.
    let attempt = normalize_prompt_text(&prompt.buffer);
    let target = normalize_prompt_text(&prompt.expected);
    if !target.is_empty() && attempt == target {
        let label = prompt.label.clone();
        let pending = prompt.pending.take();
        let entity_target = prompt.target.take();
        prompt.clear();
        sfx.send(PlaySfx(SfxKind::Confirm));
        notif.push(format!("{} confirmed.", label), 1.5);
        return pending.map(|next| (next, entity_target));
    }

    // Keep Enter as a manual fallback confirm.
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        if attempt == target {
            let label = prompt.label.clone();
            let pending = prompt.pending.take();
            let entity_target = prompt.target.take();
            prompt.clear();
            sfx.send(PlaySfx(SfxKind::Confirm));
            notif.push(format!("{} confirmed.", label), 1.5);
            return pending.map(|next| (next, entity_target));
        }

        if prompt.retries_left > 1 {
            prompt.retries_left -= 1;
            prompt.buffer.clear();
            sfx.send(PlaySfx(SfxKind::Fail));
            notif.push("Not quite - try again.".to_string(), 2.0);
        } else {
            let label = prompt.label.clone();
            prompt.clear();
            sfx.send(PlaySfx(SfxKind::Fail));
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
            stats.modify_stress(10.);
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
            stats.modify_stress(-5.);
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
            skills.gain_cooking(gain);
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
            skills.gain_cooking(gain);
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
    stats.modify_energy(-8.);
    stats.modify_happiness(-5.);
    stats.modify_stress(5.);
    skills.gain_career(0.15 * stats.skill_gain_mult());
    // Promotion check: fire once when career crosses 2.5 (Senior) and 5.0 (Executive).
    if skills.career >= 2.5 && (streak.promotion_notified & 0b01) == 0 {
        streak.promotion_notified |= 0b01;
        notif.push(
            "Promoted to Senior! Pay multiplier increased. [1.12x career bonus]".to_string(),
            6.,
        );
    } else if skills.career >= 5.0 && (streak.promotion_notified & 0b10) == 0 {
        streak.promotion_notified |= 0b10;
        notif.push(
            "Promoted to Executive! Maximum career tier reached. [1.60x career bonus]".to_string(),
            6.,
        );
    }
    gs.work_today += 1;
    gs.money_earned_today += earned;
    streak.worked_today = true;
    streak.days += 1;
    rep.add_score(0.5);
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
                stats.modify_happiness(10.);
                skills.gain_social(0.15);
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
                stats.modify_happiness(15.);
                skills.gain_fitness(0.2);
                festival.tokens += 3;
                festival.spring_attended = true;
                notif.push("Danced your heart out! +15hap +0.2fit +3 tokens", 3.);
            }
            (FestivalKind::SpringFair, _, _) => {
                if pet.has_pet {
                    rep.add_score(5.);
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
                stats.modify_hunger(-20.);
                skills.gain_cooking(0.2);
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
                skills.gain_fitness(0.3);
                stats.modify_stress(-10.);
                festival.tokens += 3;
                festival.summer_attended = true;
                notif.push("Won the Swim Race! +0.3fit -10stress +3 tokens", 3.);
            }
            (FestivalKind::SummerBBQ, _, _) => {
                stats.modify_happiness(20.);
                stats.modify_stress(-15.);
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
                hobbies.painting = (hobbies.painting + 0.2).clamp(0., 5.);
                stats.modify_happiness(10.);
                festival.tokens += 2;
                festival.autumn_attended = true;
                notif.push("Carved a pumpkin! +0.2art +10hap +2 tokens", 3.);
            }
            (FestivalKind::AutumnHarvest, _, true) => {
                inv.ingredient += 3;
                skills.gain_fitness(0.1);
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
                stats.modify_hunger(-30.);
                stats.modify_happiness(15.);
                skills.gain_cooking(0.1);
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
                skills.gain_fitness(0.2);
                stats.modify_happiness(15.);
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
                    *lvl = (*lvl + 0.5).clamp(0., 5.);
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
                rep.add_score(10.);
                stats.modify_stress(-5.);
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
    stats.modify_happiness(gain + weather_bonus + season_bonus);
    stats.modify_energy(-3.);
    stats.modify_stress(-12.);
    skills.gain_fitness(0.08);
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
            *f = (*f + friend_gain).clamp(0., 5.);
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
            stats.modify_happiness(hap);
            stats.modify_health(health_gain);
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
    *f = (*f + 0.30 * rep_friend_mult).clamp(0., 5.);
    let lvl = *f;
    friendship.chatted_today.insert(entity, true);
    stats.modify_happiness(hap);
    stats.modify_stress(-5.);
    skills.gain_social(0.15 * skill_mult * stats.skill_gain_mult());
    gs.chat_today += 1;
    stats.cooldown = 1.5;
    let base_rep = 0.8;
    let rep_bonus = if personality == NpcPersonality::Influential {
        3.0
    } else {
        0.0
    };
    rep.add_score(base_rep + rep_bonus);
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
    desk_mult: f32,
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
    let study_gain = (0.5 + season_bonus) * stats.skill_gain_mult() * rainy_study_mult * desk_mult;
    let (boost_name, new_lvl) = match seed {
        0 => {
            skills.gain_cooking(study_gain);
            ("Cooking", skills.cooking)
        }
        1 => {
            skills.gain_career(study_gain);
            ("Career", skills.career)
        }
        2 => {
            skills.gain_fitness(study_gain);
            ("Fitness", skills.fitness)
        }
        _ => {
            skills.gain_social(study_gain);
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

#[allow(clippy::type_complexity)]
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
            &mut PlayerStats,
            &mut Inventory,
            &mut Skills,
            &mut WorkStreak,
            &mut HousingTier,
            &mut Furnishings,
        ),
        With<LocalPlayer>,
    >,
    mut gt: ResMut<GameTime>,
    mut friendship: ResMut<NpcFriendship>,
    mut gs: ResMut<GameState>,
    mut notif: ResMut<Notification>,
    mut extras: InteractExtras,
    mut sfx: EventWriter<PlaySfx>,
) {
    let Some((
        mut pm,
        mut vehicle_state,
        mut bank_input,
        mut action_prompt,
        mut stats,
        mut inv,
        mut skills,
        mut streak,
        mut housing,
        mut furnishings,
    )) = player_q.iter_mut().next()
    else {
        return;
    };
    let mut pe = keys.just_pressed(KeyCode::KeyE);
    let mut pg = keys.just_pressed(KeyCode::KeyG);
    let ph = keys.just_pressed(KeyCode::KeyH);
    let pf1 = keys.just_pressed(KeyCode::F1);
    let pf2 = keys.just_pressed(KeyCode::F2);
    let pf3 = keys.just_pressed(KeyCode::F3);
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
        handle_action_prompt_input(&keys, &mut action_prompt, &mut notif, &mut sfx)
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

    // ── H key: begin hangout prompt if NPC nearby ─────────────────────────────
    if ph && forced_action.is_none() && matches!(&inter.action, ActionKind::Chat) {
        let lvl = friendship.levels.get(&entity).copied().unwrap_or(0.);
        if lvl < 3. {
            notif.push(
                format!("Need friendship level 3 to hang out (current: {:.1}).", lvl),
                2.5,
            );
            return;
        }
        pe = true;
        forced_action = Some(ActionKind::Hangout);
    }

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
            PendingAction::Action(ActionKind::Chat)
                | PendingAction::Action(ActionKind::Hangout)
                | PendingAction::Gift
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
            let sleep_bonus = furnishings.sleep_bonus();
            let health_gain = housing.night_health();
            stats.modify_energy(gain + sleep_bonus);
            stats.modify_health(health_gain + 3.);
            stats.modify_stress(-15.);
            stats.sleep_debt = (stats.sleep_debt - 8.).max(0.);
            stats.cooldown = 3.;
            let tag = if gt.is_night() {
                "Night sleep"
            } else {
                "Daytime nap"
            };
            let bonus_tag = if sleep_bonus > 0. {
                " [Comfy Bed +10]"
            } else {
                ""
            };
            notif.push(
                format!(
                    "{} — +{:.0} Energy, -SleepDebt, -Stress{}{}",
                    tag,
                    gain + sleep_bonus,
                    stress_tag,
                    bonus_tag
                ),
                2.,
            );
        }
        ActionKind::Eat => {
            sfx_kind = SfxKind::Eat;
            let reduction = 40. * skills.cooking_bonus();
            let meal_bonus = furnishings.meal_bonus();
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
            stats.modify_hunger(-(reduction + meal_bonus));
            stats.modify_health(1. + extra_health);
            stats.modify_happiness(extra_hap);
            stats.modify_energy(breakfast_bonus);
            skills.gain_cooking(0.10 * stats.skill_gain_mult() * furnishings.skill_mult());
            gs.eat_today += 1;
            stats.cooldown = 2.;
            let bfast = if breakfast_bonus > 0. {
                " [Breakfast +10 Energy!]"
            } else {
                ""
            };
            let kitchen_tag = if meal_bonus > 0. {
                " [Kitchen +10]"
            } else {
                ""
            };
            notif.push(
                format!(
                    "{} — -{:.0} Hunger{}{}",
                    meal_label,
                    reduction + meal_bonus,
                    bfast,
                    kitchen_tag
                ),
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
            stats.modify_energy(-8.);
            stats.modify_stress(3.);
            gs.work_today += 1;
            gs.money_earned_today += earned;
            streak.worked_today = true;
            extras.rep.add_score(0.3);
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
            stats.modify_energy(-exercise_cost);
            stats.modify_health(8. * skills.fitness_bonus());
            stats.modify_hunger(10.);
            stats.modify_happiness(10.);
            stats.modify_stress(-8.);
            skills.gain_fitness(fit_gain);
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
            stats.modify_happiness(hap_gain + fish_hap);
            stats.modify_stress(-25. - fish_stress);
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
            stats.modify_happiness(12.);
            stats.modify_health(2.);
            stats.modify_stress(-5.);
            stats.cooldown = 2.;
            notif.push("Showered! +12 Happiness, +Health, -Stress.", 2.);
        }
        ActionKind::Bank => {
            // ── Furnishing purchases (F1/F2/F3) ──────────────────────────────
            if pf1 || pf2 || pf3 {
                if !housing.has_access() {
                    notif.push("Need an apartment before buying furnishings.", 2.5);
                    stats.cooldown = 0.5;
                } else if pf1 {
                    if furnishings.desk {
                        notif.push("Desk already owned.", 2.);
                    } else if stats.savings >= 60. {
                        stats.savings -= 60.;
                        furnishings.desk = true;
                        notif.push("Desk purchased! +15% skill XP.", 4.);
                    } else {
                        notif.push("Need $60 savings for a Desk.", 2.5);
                    }
                    stats.cooldown = 0.5;
                } else if pf2 {
                    if furnishings.bed {
                        notif.push("Comfy Bed already owned.", 2.);
                    } else if stats.savings >= 80. {
                        stats.savings -= 80.;
                        furnishings.bed = true;
                        notif.push("Comfy Bed purchased! +10 energy on each sleep.", 4.);
                    } else {
                        notif.push("Need $80 savings for a Comfy Bed.", 2.5);
                    }
                    stats.cooldown = 0.5;
                } else {
                    if furnishings.kitchen {
                        notif.push("Kitchen Upgrade already owned.", 2.);
                    } else if stats.savings >= 100. {
                        stats.savings -= 100.;
                        furnishings.kitchen = true;
                        notif.push("Kitchen upgraded! +10 hunger reduction on each meal.", 4.);
                    } else {
                        notif.push("Need $100 savings for a Kitchen Upgrade.", 2.5);
                    }
                    stats.cooldown = 0.5;
                }
                return;
            }
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
                            "Bank: ${:.0} saved. [1]Dep [2]Wth [3]HalfDep [4]Loan [5]Repay [6]Invest(lo) [7]Invest(md) [8]CashOut [9]Insurance. Upgrade: ${:.0} for {}. [F1]Desk$60 [F2]Bed$80 [F3]Kitchen$100",
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
                    format!("Bank: ${:.0} saved. Max housing! [1-8] [F1]Desk$60 [F2]Bed$80 [F3]Kitchen$100", stats.savings),
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
                        stats.modify_energy(30.);
                        stats.cooldown = 1.;
                        notif.message = format!("Coffee! +30 Energy. ({}x left)", inv.coffee);
                    } else {
                        notif.message = "No coffee!".to_string();
                    }
                }
                ItemKind::Vitamins => {
                    if inv.vitamins > 0 {
                        inv.vitamins -= 1;
                        stats.modify_health(15.);
                        stats.cooldown = 1.;
                        notif.message = format!("Vitamins! +15 Health. ({}x left)", inv.vitamins);
                    } else {
                        notif.message = "No vitamins!".to_string();
                    }
                }
                ItemKind::Books => {
                    if inv.books > 0 {
                        inv.books -= 1;
                        skills.gain_career(0.5);
                        stats.cooldown = 2.;
                        notif.message = format!("Read! +0.5 Career XP. ({}x left)", inv.books);
                    } else {
                        notif.message = "No books!".to_string();
                    }
                }
                ItemKind::Smoothie => {
                    if inv.smoothie > 0 {
                        inv.smoothie -= 1;
                        stats.modify_energy(40.);
                        stats.modify_health(10.);
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
            *skill_val = (*skill_val + 0.25 * stats.skill_gain_mult() * rainy_mult).clamp(0., 5.);
            let lvl = *skill_val;
            let winter_bonus = extras.season.current.indoor_bonus();
            stats.modify_happiness(12. + winter_bonus);
            stats.modify_stress(-8.);
            stats.modify_energy(-10.);
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
                furnishings.skill_mult(),
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
            stats.modify_happiness(15.);
            stats.modify_stress(-5.);
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
            stats.modify_happiness(30.);
            stats.modify_stress(-10.);
            extras.social_events.parties_thrown += 1;
            extras.social_events.party_today = true;
            extras.rep.add_score(5.);
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
        ActionKind::Hangout => {
            let lvl = friendship.levels.get(&entity).copied().unwrap_or(0.);
            if lvl < 3. {
                notif.push(
                    format!("Need friendship level 3 to hang out (current: {:.1}).", lvl),
                    2.5,
                );
                return;
            }
            let npc_name = npc_q
                .get(entity)
                .ok()
                .map(|(n, _)| n.name.clone())
                .unwrap_or_else(|| "them".to_string());
            let f = friendship.levels.entry(entity).or_insert(0.);
            *f = (*f + 0.5).clamp(0., 5.);
            stats.modify_happiness(25.);
            stats.modify_stress(-10.);
            stats.modify_energy(-8.);
            skills.gain_social(0.2 * stats.skill_gain_mult());
            extras.rep.add_score(1.0);
            gs.chat_today += 1;
            stats.cooldown = 2.;
            notif.push(
                format!(
                    "Hung out with {}! +0.5 friendship, +25 happiness, -10 stress.",
                    npc_name
                ),
                3.,
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
            stats.modify_energy(-25.);
            stats.modify_health(12. * skills.fitness_bonus());
            stats.modify_hunger(12.);
            stats.modify_happiness(12.);
            stats.modify_stress(-12.);
            skills.gain_fitness(fit_gain);
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
            stats.energy = (stats.energy + 25.).clamp(0., 100.);
            stats.modify_happiness(12.);
            stats.modify_hunger(-25.);
            extras.rep.add_score(2.);
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
            stats.modify_health(35.);
            stats.modify_stress(-8.);
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
            stats.modify_happiness(20.);
            stats.cooldown = 1.;
            notif.push(
                format!("You adopted {}! Feed daily for bonuses.", extras.pet.name),
                4.,
            );
        }
        ActionKind::SleepRough => {
            stats.modify_energy(25.);
            stats.modify_health(2.);
            stats.modify_stress(5.);
            stats.modify_happiness(-5.);
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
                stats.modify_health(15.);
                stats.modify_stress(-5.);
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
                stats.modify_stress(-8.);
                stats.modify_happiness(8.);
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
                furnishings.skill_mult(),
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
    mut player_bank_q: Query<&mut BankInput, With<LocalPlayer>>,
    mut player_stats_q: Query<&mut PlayerStats, With<LocalPlayer>>,
    mut notif: ResMut<Notification>,
    mut gt: ResMut<GameTime>,
    mut goal: ResMut<DailyGoal>,
) {
    let Some(mut bank_input) = player_bank_q.iter_mut().next() else {
        return;
    };
    if !bank_input.active {
        return;
    }
    let Some(mut stats) = player_stats_q.iter_mut().next() else {
        return;
    };

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
                    stats.modify_stress(-3.);
                    gt.advance_hours(0.25);
                    if matches!(&goal.kind, GoalKind::SaveMoney) && !goal.completed {
                        goal.progress = stats.savings;
                        if goal.progress >= goal.target {
                            goal.completed = true;
                            stats.money += goal.reward_money;
                            stats.modify_happiness(goal.reward_happiness);
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

    // ── pick_word ─────────────────────────────────────────────────────────────

    #[test]
    fn pick_word_returns_pool_member() {
        let pool: &[&str] = &["alpha", "beta", "gamma"];
        let word = pick_word(pool, 42, 0);
        assert!(pool.contains(&word));
    }

    #[test]
    fn pick_word_wraps_on_large_seed() {
        let pool: &[&str] = &["only"];
        assert_eq!(pick_word(pool, u32::MAX, 0), "only");
        assert_eq!(pick_word(pool, u32::MAX, 999), "only");
    }

    #[test]
    fn pick_word_offset_changes_selection() {
        let pool: &[&str] = &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"];
        // With a large enough pool and two different offsets the index calculation
        // (seed + offset * 11) % len will differ when offset differs by 1.
        let w0 = pick_word(pool, 0, 0);
        let w1 = pick_word(pool, 0, 1);
        // offset=0 -> index 0 % 12 = 0 => "a"
        // offset=1 -> index 11 % 12 = 11 => "l"
        assert_ne!(w0, w1);
    }

    #[test]
    fn pick_word_deterministic_same_inputs() {
        let pool = WORK_WORDS;
        assert_eq!(pick_word(pool, 7, 3), pick_word(pool, 7, 3));
    }

    // ── normalize_prompt_text ─────────────────────────────────────────────────

    #[test]
    fn normalize_lowercases_input() {
        assert_eq!(normalize_prompt_text("HELLO"), "hello");
        assert_eq!(normalize_prompt_text("MiXeD"), "mixed");
    }

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize_prompt_text("  hello   world  "), "hello world");
    }

    #[test]
    fn normalize_empty_string_stays_empty() {
        assert_eq!(normalize_prompt_text(""), "");
        assert_eq!(normalize_prompt_text("   "), "");
    }

    #[test]
    fn normalize_auto_confirm_condition() {
        // Simulates the check inside handle_action_prompt_input.
        let buffer = "Deadline";
        let expected = "deadline";
        assert_eq!(
            normalize_prompt_text(buffer),
            normalize_prompt_text(expected)
        );
    }

    // ── word_challenge ────────────────────────────────────────────────────────

    #[test]
    fn word_challenge_returns_single_word() {
        let c = word_challenge("Work", "type to work", WORK_WORDS, 0);
        assert_eq!(c.expected.split_whitespace().count(), 1);
    }

    #[test]
    fn word_challenge_expected_is_pool_member() {
        let c = word_challenge("Eat", "type to eat", EAT_WORDS, 5);
        assert!(EAT_WORDS.contains(&c.expected.as_str()));
    }

    #[test]
    fn word_challenge_preserves_label_and_instruction() {
        let c = word_challenge("Sleep", "type to sleep", SLEEP_WORDS, 99);
        assert_eq!(c.label, "Sleep");
        assert_eq!(c.instruction, "type to sleep");
    }

    // ── build_prompt_challenge ────────────────────────────────────────────────

    #[test]
    fn work_prompt_returns_single_word() {
        let c = build_prompt_challenge(&PendingAction::Action(ActionKind::Work), 0, "buddy");
        assert_eq!(c.expected.split_whitespace().count(), 1);
        assert!(WORK_WORDS.contains(&c.expected.as_str()));
        assert!(c.label.contains("Work"));
    }

    #[test]
    fn eat_prompt_picks_from_eat_words() {
        let c = build_prompt_challenge(&PendingAction::Action(ActionKind::Eat), 7, "buddy");
        assert!(EAT_WORDS.contains(&c.expected.as_str()));
        assert!(c.label.contains("Eat"));
    }

    #[test]
    fn chat_prompt_uses_subject_name() {
        let challenge = build_prompt_challenge(&PendingAction::Action(ActionKind::Chat), 3, "alex");
        assert!(challenge.expected.contains("alex"));
    }

    #[test]
    fn challenge_expected_is_nonempty_for_all_basic_actions() {
        let actions = [
            ActionKind::Work,
            ActionKind::Eat,
            ActionKind::Sleep,
            ActionKind::Shop,
            ActionKind::Relax,
            ActionKind::Shower,
            ActionKind::Exercise,
            ActionKind::Meditate,
            ActionKind::StudyCourse,
            ActionKind::Freelance,
        ];
        for action in &actions {
            let c = build_prompt_challenge(&PendingAction::Action(action.clone()), 42, "buddy");
            assert!(!c.expected.is_empty());
            assert!(!c.label.is_empty());
        }
    }
}
