#[allow(dead_code)]
pub const MAP_SCALE: f32 = 4.0;
pub const PLAYER_SPEED: f32 = 800.0;
pub const PLAYER_ACCEL: f32 = 5600.0;
pub const PLAYER_FRICTION: f32 = 3600.0;
pub const SPRINT_MULTIPLIER: f32 = 1.75;
pub const SPRINT_ENERGY_DRAIN: f32 = 3.5;
pub const WORLD_BOUNDARY: f32 = 6400.0;
pub const INTERACT_RADIUS: f32 = 232.0;
pub const TIME_SCALE: f32 = 60.0;
pub const NPC_SPEED: f32 = 220.0;

// Office zone — center (1700, 1480), size 600×720 (scaled 4x)
pub const OFFICE_LEFT: f32 = 1400.;
pub const OFFICE_RIGHT: f32 = 2000.;
pub const OFFICE_BOTTOM: f32 = 400.;
pub const OFFICE_TOP: f32 = 1040.;

// Crisis system tuning
pub const CRISIS_MIN_DAY: u32 = 10;
pub const CRISIS_COOLDOWN_DAYS: u32 = 5;
pub const CRISIS_CHANCE_EASY: u64 = 4;
pub const CRISIS_CHANCE_NORMAL: u64 = 8;
pub const CRISIS_CHANCE_HARD: u64 = 12;
pub const CRISIS_NOTIF_DURATION: f32 = 8.;
pub const CRISIS_RESOLVE_NOTIF_DURATION: f32 = 6.;
pub const CRISIS_STRESS_INCREASE: f32 = 20.;
pub const CRISIS_HAPPINESS_DECREASE: f32 = 15.;
pub const INSURANCE_DAMAGE_MULTIPLIER: f32 = 0.5;
pub const MARKET_CRASH_BASE_LOSS_PCT: f32 = 0.40;
pub const MARKET_CRASH_RANDOM_LOSS_RANGE: u64 = 20;
pub const MARKET_CRASH_SAVINGS_LOSS_PCT: f32 = 0.15;
pub const MEDICAL_BILL_AMOUNT: f32 = 200.;
pub const MEDICAL_HEALTH_FLOOR: f32 = 20.;
pub const MEDICAL_ENERGY_DRAIN: f32 = 30.;
pub const THEFT_CASH_FRACTION: f32 = 0.3;
pub const MIN_INVESTMENT_FOR_CRASH: f32 = 1.;
