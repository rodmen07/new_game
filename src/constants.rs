// MAP_SCALE multiplies design (pre-scale) coordinates to produce world-space
// coordinates. The bevy_ecs_tilemap atlas is tuned around MAP_SCALE = 4.0;
// changing it requires retuning TILE_PX/TILEMAP_ORIGIN_* in setup.rs.
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

// Office zone (design center 425, 180; size 180x220) in world space.
const OFFICE_DESIGN_CX: f32 = 425.;
const OFFICE_DESIGN_CY: f32 = 180.;
const OFFICE_DESIGN_HW: f32 = 90.; // 180 / 2
const OFFICE_DESIGN_HH: f32 = 110.; // 220 / 2
pub const OFFICE_LEFT: f32 = (OFFICE_DESIGN_CX - OFFICE_DESIGN_HW) * MAP_SCALE;
pub const OFFICE_RIGHT: f32 = (OFFICE_DESIGN_CX + OFFICE_DESIGN_HW) * MAP_SCALE;
pub const OFFICE_BOTTOM: f32 = (OFFICE_DESIGN_CY - OFFICE_DESIGN_HH) * MAP_SCALE;
pub const OFFICE_TOP: f32 = (OFFICE_DESIGN_CY + OFFICE_DESIGN_HH) * MAP_SCALE;

// Player home zone (design center -425, 180; size 180x220) in world space.
// Used by NPC pathing to keep wandering NPCs out of the player's house.
const PLAYER_HOME_DESIGN_CX: f32 = -425.;
const PLAYER_HOME_DESIGN_CY: f32 = 180.;
const PLAYER_HOME_DESIGN_HW: f32 = 90.;
const PLAYER_HOME_DESIGN_HH: f32 = 110.;
pub const PLAYER_HOME_LEFT: f32 = (PLAYER_HOME_DESIGN_CX - PLAYER_HOME_DESIGN_HW) * MAP_SCALE;
pub const PLAYER_HOME_RIGHT: f32 = (PLAYER_HOME_DESIGN_CX + PLAYER_HOME_DESIGN_HW) * MAP_SCALE;
pub const PLAYER_HOME_BOTTOM: f32 = (PLAYER_HOME_DESIGN_CY - PLAYER_HOME_DESIGN_HH) * MAP_SCALE;
pub const PLAYER_HOME_TOP: f32 = (PLAYER_HOME_DESIGN_CY + PLAYER_HOME_DESIGN_HH) * MAP_SCALE;

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
pub const DEBT_LIMIT: f32 = 1000.;
