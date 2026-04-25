use crate::{
    components::{ActionKind, LocalPlayer, PetKind},
    constants::DEBT_LIMIT,
    save::SaveRequest,
    settings::GameSettings,
};
use bevy::{ecs::system::SystemParam, prelude::*, utils::HashMap};
use std::collections::VecDeque;

/// Optional handles to art assets that override the procedurally generated
/// look. None means "use built-in procedural art" (current default). When
/// pixel-art tilesets and character spritesheets are added under
/// `assets/art/`, populate these handles in `setup()` to swap them in
/// without touching gameplay code.
///
/// Naming convention for future assets:
/// - `art/tiles/terrain.png` (18-column tilesheet matching TILE_COLORS)
/// - `art/characters/player.png` (4 rows: south/north/east/west, 4 cols)
/// - `art/characters/npc_<name>.png` (same layout per NPC)
#[derive(Resource, Default, Clone)]
#[allow(dead_code)]
pub struct ArtAssets {
    pub tile_atlas: Option<Handle<Image>>,
    pub player_sheet: Option<Handle<Image>>,
    pub player_atlas: Option<Handle<TextureAtlasLayout>>,
    pub npc_sheets: HashMap<String, Handle<Image>>,
}

#[derive(Resource, Default)]
pub struct NearbyInteractable {
    pub entity: Option<Entity>,
    pub prompt: String,
}

// ── PlayerMovement ────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct PlayerMovement {
    pub velocity: Vec2,
    pub sprinting: bool,
    pub base_zoom: f32,
    pub prev_position: Vec2,
}
impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            velocity: Vec2::ZERO,
            sprinting: false,
            base_zoom: 4.0,
            prev_position: Vec2::ZERO,
        }
    }
}

// ── PlayerStats ───────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct PlayerStats {
    pub energy: f32,
    pub hunger: f32,
    pub happiness: f32,
    pub health: f32,
    pub stress: f32,
    pub sleep_debt: f32,
    pub money: f32,
    pub savings: f32,
    pub loan: f32,
    pub meals: u32,
    pub cooldown: f32,
    pub critical_timer: f32,
    pub unpaid_rent_days: u32,
    pub meditation_buff: f32,
}
impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            energy: 80.,
            hunger: 15.,
            happiness: 55.,
            health: 92.,
            stress: 25.,
            sleep_debt: 2.,
            money: 80.,
            savings: 0.,
            loan: 0.,
            meals: 4,
            cooldown: 0.,
            critical_timer: 0.,
            unpaid_rent_days: 0,
            meditation_buff: 0.,
        }
    }
}
impl PlayerStats {
    pub fn max_energy(&self) -> f32 {
        if self.sleep_debt > 16. {
            60.
        } else if self.sleep_debt > 8. {
            80.
        } else {
            100.
        }
    }
    pub fn stress_work_mult(&self) -> f32 {
        if self.stress > 75. {
            0.50
        } else if self.stress > 50. {
            0.85
        } else {
            1.0
        }
    }
    pub fn loan_penalty(&self) -> f32 {
        if self.loan > 300. { 0.90 } else { 1.0 }
    }
    /// Returns true if the player can afford `cost`, counting up to DEBT_LIMIT in credit.
    pub fn can_afford(&self, cost: f32) -> bool {
        self.money + DEBT_LIMIT >= cost
    }
    /// Skill XP multiplier based on sleep debt.
    /// Sleep-deprived players learn more slowly: -20% at moderate debt, -40% at severe.
    pub fn skill_gain_mult(&self) -> f32 {
        if self.sleep_debt > 16. {
            0.60
        } else if self.sleep_debt > 8. {
            0.80
        } else {
            1.0
        }
    }

    // ── Bounded stat mutators ─────────────────────────────────────────────────
    pub fn modify_health(&mut self, delta: f32) {
        self.health = (self.health + delta).clamp(0., 100.);
    }
    pub fn modify_energy(&mut self, delta: f32) {
        let max = self.max_energy();
        self.energy = (self.energy + delta).clamp(0., max);
    }
    pub fn modify_happiness(&mut self, delta: f32) {
        self.happiness = (self.happiness + delta).clamp(0., 100.);
    }
    pub fn modify_stress(&mut self, delta: f32) {
        self.stress = (self.stress + delta).clamp(0., 100.);
    }
    pub fn modify_hunger(&mut self, delta: f32) {
        self.hunger = (self.hunger + delta).clamp(0., 100.);
    }
}

#[derive(Component, Default)]
pub struct Inventory {
    pub coffee: u32,
    pub vitamins: u32,
    pub books: u32,
    pub coffee_age: u32, // days since last purchase; coffee expires after 5 days
    pub ingredient: u32,
    pub gift_box: u32,
    pub smoothie: u32,
}

// ── GameTime ──────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GameTime {
    pub hours: f32,
    pub day: u32,
    pub prev_day: u32,
    /// Accumulates only while in the Playing state; use this for animations.
    pub anim_secs: f32,
}
impl Default for GameTime {
    fn default() -> Self {
        Self {
            hours: 8.,
            day: 0,
            prev_day: 0,
            anim_secs: 0.,
        }
    }
}
impl GameTime {
    pub fn display(&self) -> String {
        let h = self.hours as u32 % 24;
        let m = (self.hours.fract() * 60.) as u32;
        let (ampm, h12) = if h < 12 {
            ("AM", if h == 0 { 12 } else { h })
        } else {
            ("PM", if h == 12 { 12 } else { h - 12 })
        };
        format!(
            "Day {}  {:02}:{:02} {}  ({})",
            self.day + 1,
            h12,
            m,
            ampm,
            self.day_name()
        )
    }
    pub fn is_night(&self) -> bool {
        self.hours >= 21. || self.hours < 6.
    }
    pub fn is_weekend(&self) -> bool {
        self.day % 7 >= 5
    }
    pub fn day_name(&self) -> &str {
        match self.day % 7 {
            0 => "Mon",
            1 => "Tue",
            2 => "Wed",
            3 => "Thu",
            4 => "Fri",
            5 => "Sat",
            _ => "Sun",
        }
    }
    pub fn work_time_tag(&self) -> (f32, &str) {
        let h = self.hours as u32;
        if (6..10).contains(&h) {
            (1.15, " [Early Bird +15%]")
        } else if h >= 20 {
            (0.90, " [Late Night -10%]")
        } else {
            (1.0, "")
        }
    }
    pub fn exercise_mult(&self) -> f32 {
        let h = self.hours as u32;
        if (5..9).contains(&h) { 1.25 } else { 1.0 }
    }
    pub fn is_breakfast(&self) -> bool {
        let h = self.hours as u32;
        (6..9).contains(&h)
    }
}

// ── Skills ────────────────────────────────────────────────────────────────────

#[derive(Component, Default)]
pub struct Skills {
    pub cooking: f32,
    pub career: f32,
    pub fitness: f32,
    pub social: f32,
}
impl Skills {
    pub fn cooking_bonus(&self) -> f32 {
        1. + self.cooking * 0.10
    }
    pub fn career_bonus(&self) -> f32 {
        1. + self.career * 0.12
    }
    pub fn social_bonus(&self) -> f32 {
        1. + self.social * 0.10
    }
    pub fn fitness_bonus(&self) -> f32 {
        1. + self.fitness * 0.05
    }
    pub fn career_rank(&self) -> &str {
        if self.career >= 5.0 {
            "Executive"
        } else if self.career >= 2.5 {
            "Senior"
        } else {
            "Junior"
        }
    }
    // ── Bounded skill mutators ────────────────────────────────────────────────
    pub fn gain_cooking(&mut self, amount: f32) {
        self.cooking = (self.cooking + amount).clamp(0., 5.);
    }
    pub fn gain_career(&mut self, amount: f32) {
        self.career = (self.career + amount).clamp(0., 5.);
    }
    pub fn gain_fitness(&mut self, amount: f32) {
        self.fitness = (self.fitness + amount).clamp(0., 5.);
    }
    pub fn gain_social(&mut self, amount: f32) {
        self.social = (self.social + amount).clamp(0., 5.);
    }

    pub fn work_pay(&self, streak: u32) -> f32 {
        let base = if self.career >= 5.0 {
            70.
        } else if self.career >= 2.5 {
            45.
        } else {
            30.
        };
        let s = if streak >= 7 {
            1.35
        } else if streak >= 5 {
            1.20
        } else if streak >= 3 {
            1.10
        } else {
            1.0
        };
        base * self.career_bonus() * s
    }
}

// ── WorkStreak ────────────────────────────────────────────────────────────────

#[derive(Component, Default)]
pub struct WorkStreak {
    pub days: u32,
    pub worked_today: bool,
    /// Bitmask: bit 0 = Senior promotion shown, bit 1 = Executive promotion shown.
    pub promotion_notified: u8,
}

// ── HousingTier ───────────────────────────────────────────────────────────────

#[derive(Component, Default, Clone, PartialEq)]
pub enum HousingTier {
    #[default]
    Unhoused,
    Apartment,
    Condo,
    Penthouse,
}
impl From<&HousingTier> for u8 {
    fn from(h: &HousingTier) -> u8 {
        match h {
            HousingTier::Unhoused => 0,
            HousingTier::Apartment => 1,
            HousingTier::Condo => 2,
            HousingTier::Penthouse => 3,
        }
    }
}
impl From<u8> for HousingTier {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::Apartment,
            2 => Self::Condo,
            3 => Self::Penthouse,
            _ => Self::Unhoused,
        }
    }
}
impl HousingTier {
    pub fn has_access(&self) -> bool {
        !matches!(self, Self::Unhoused)
    }

    pub fn rent(&self) -> f32 {
        match self {
            Self::Unhoused => 0.,
            Self::Apartment => 30.,
            Self::Condo => 50.,
            Self::Penthouse => 85.,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Unhoused => "No Lease",
            Self::Apartment => "Apartment",
            Self::Condo => "Condo",
            Self::Penthouse => "Penthouse",
        }
    }
    pub fn upgrade_cost(&self) -> Option<f32> {
        match self {
            Self::Unhoused => Some(90.),
            Self::Apartment => Some(350.),
            Self::Condo => Some(800.),
            Self::Penthouse => None,
        }
    }
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Unhoused => Some(Self::Apartment),
            Self::Apartment => Some(Self::Condo),
            Self::Condo => Some(Self::Penthouse),
            Self::Penthouse => None,
        }
    }
    pub fn sleep_energy(&self, night: bool) -> f32 {
        match self {
            Self::Unhoused => {
                if night {
                    20.
                } else {
                    8.
                }
            }
            Self::Apartment => {
                if night {
                    65.
                } else {
                    30.
                }
            }
            Self::Condo => {
                if night {
                    82.
                } else {
                    42.
                }
            }
            Self::Penthouse => {
                if night {
                    100.
                } else {
                    55.
                }
            }
        }
    }
    pub fn night_health(&self) -> f32 {
        match self {
            Self::Condo => 5.,
            Self::Penthouse => 10.,
            _ => 0.,
        }
    }
    pub fn morning_hap(&self) -> f32 {
        match self {
            Self::Penthouse => 8.,
            Self::Condo => 3.,
            _ => 0.,
        }
    }
}

// ── NpcFriendship ─────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct NpcFriendship {
    pub levels: HashMap<Entity, f32>,
    pub chatted_today: HashMap<Entity, bool>,
    pub gifted_today: HashMap<Entity, bool>,
}

// ── Quest Board ───────────────────────────────────────────────────────────────

use crate::components::QuestKind;

#[derive(Clone)]
pub struct NpcQuest {
    pub npc_id: usize,
    pub kind: QuestKind,
    pub description: String,
    pub reward_money: f32,
    pub reward_friendship: f32,
    pub progress: u32,
    pub target: u32,
    pub completed: bool,
}

#[derive(Resource, Default)]
pub struct QuestBoard {
    pub quests: Vec<NpcQuest>,
    pub completed_total: u32,
    pub crafted_today: u32,
}
impl QuestBoard {
    pub fn active_count(&self) -> usize {
        self.quests.iter().filter(|q| !q.completed).count()
    }
    pub fn has_quest_from(&self, npc_id: usize) -> bool {
        self.quests
            .iter()
            .any(|q| q.npc_id == npc_id && !q.completed)
    }
}

// ── Misc Resources ────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct Notification {
    pub message: String,
    pub timer: f32,
    queue: VecDeque<(String, f32)>,
}
impl Default for Notification {
    fn default() -> Self {
        Self {
            message: String::new(),
            timer: 0.,
            queue: VecDeque::new(),
        }
    }
}
impl Notification {
    /// Enqueue a notification. Displays immediately if nothing is showing,
    /// otherwise queues up to 4 pending messages (oldest extras are dropped).
    pub fn push(&mut self, message: impl Into<String>, duration: f32) {
        if self.timer <= 0. {
            self.message = message.into();
            self.timer = duration;
        } else if self.queue.len() < 4 {
            self.queue.push_back((message.into(), duration));
        }
    }

    /// Take the current `message` contents and re-push them through the queue
    /// with the given duration. Use this after setting `notif.message = ...`
    /// directly, so the message gets proper timer/queue handling.
    pub fn flush_message(&mut self, duration: f32) {
        let msg = std::mem::take(&mut self.message);
        if !msg.is_empty() {
            self.push(msg, duration);
        }
    }

    /// Advance the display timer and pop the next queued message when ready.
    /// Call once per frame from a Playing-state system.
    pub fn tick(&mut self, dt: f32) {
        if self.timer > 0. {
            self.timer -= dt;
            if self.timer <= 0.
                && let Some((msg, dur)) = self.queue.pop_front()
            {
                self.message = msg;
                self.timer = dur;
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct NarrativeState {
    pub current_title: String,
    pub latest_entry: String,
    pub entries: Vec<String>,
    pub unlocked: Vec<String>,
}

impl NarrativeState {
    pub fn unlock(&mut self, key: &str, title: &str, body: &str) -> bool {
        if self.unlocked.iter().any(|k| k == key) {
            return false;
        }
        self.unlocked.push(key.to_string());
        self.current_title = title.to_string();
        self.latest_entry = body.to_string();
        self.entries.push(format!("{}: {}", title, body));
        true
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn latest_summary(&self) -> String {
        if self.entries.is_empty() {
            "Your story is just beginning. Build a routine and see who you become.".to_string()
        } else {
            format!("{}\n{}", self.current_title, self.latest_entry)
        }
    }
}

#[derive(Resource, Default)]
pub struct LifeRating {
    pub score: f32,
    pub days: u32,
}
impl LifeRating {
    pub fn sample(&mut self, s: &PlayerStats, sk: &Skills) {
        let v = (s.energy * 0.10
            + (100. - s.hunger) * 0.10
            + s.happiness * 0.20
            + s.health * 0.15
            + (100. - s.stress) * 0.08
            + (s.money.min(500.) / 500.) * 100. * 0.08
            + (s.savings.min(1000.) / 1000.) * 100. * 0.12
            + (sk.career + sk.cooking + sk.fitness + sk.social) / 20. * 100. * 0.12
            - s.loan.min(500.) / 500. * 15.)
            .clamp(0., 100.);
        self.score = (self.score * self.days as f32 + v) / (self.days + 1) as f32;
        self.days += 1;
    }
    pub fn grade(&self) -> &str {
        match self.score as u32 {
            90..=100 => "S — Thriving",
            75..=89 => "A — Flourishing",
            60..=74 => "B — Comfortable",
            45..=59 => "C — Getting By",
            30..=44 => "D — Struggling",
            _ => "F — Crisis",
        }
    }
}

#[derive(Resource, Default)]
pub struct Milestones {
    pub saved_100: bool,
    pub exec: bool,
    pub best_friend: bool,
    pub streak_7: bool,
    pub rating_a: bool,
    pub debt_free: bool,
    pub penthouse: bool,
    pub investor: bool,
    pub hobbyist: bool,
    pub famous: bool,
    pub scholar: bool,
    pub pet_owner: bool,
    pub party_animal: bool,
    pub commuter: bool,
    pub all_seasons: bool,
    pub quest_master: bool,
    pub master_chef: bool,
    pub gift_giver: bool,
    pub popular: bool,
    pub crisis_survivor: bool,
    pub festival_goer: bool,
}
impl Milestones {
    pub fn count(&self) -> u32 {
        [
            self.saved_100,
            self.exec,
            self.best_friend,
            self.streak_7,
            self.rating_a,
            self.debt_free,
            self.penthouse,
            self.investor,
            self.hobbyist,
            self.famous,
            self.scholar,
            self.pet_owner,
            self.party_animal,
            self.commuter,
            self.all_seasons,
            self.quest_master,
            self.master_chef,
            self.gift_giver,
            self.popular,
            self.crisis_survivor,
            self.festival_goer,
        ]
        .iter()
        .filter(|&&b| b)
        .count() as u32
    }
    pub fn summary(&self) -> String {
        let mut v: Vec<&str> = vec![];
        if self.saved_100 {
            v.push("Saver");
        }
        if self.exec {
            v.push("Exec");
        }
        if self.best_friend {
            v.push("BFF");
        }
        if self.streak_7 {
            v.push("Streak7");
        }
        if self.rating_a {
            v.push("Flourishing");
        }
        if self.debt_free {
            v.push("Debt-Free");
        }
        if self.penthouse {
            v.push("Penthouse");
        }
        if self.investor {
            v.push("Investor");
        }
        if self.hobbyist {
            v.push("Hobbyist");
        }
        if self.famous {
            v.push("Famous");
        }
        if self.scholar {
            v.push("Scholar");
        }
        if self.pet_owner {
            v.push("Pet Owner");
        }
        if self.party_animal {
            v.push("Party Animal");
        }
        if self.commuter {
            v.push("Commuter");
        }
        if self.all_seasons {
            v.push("All Seasons");
        }
        if self.quest_master {
            v.push("Quest Master");
        }
        if self.master_chef {
            v.push("Master Chef");
        }
        if self.gift_giver {
            v.push("Gift Giver");
        }
        if self.popular {
            v.push("Popular");
        }
        if self.crisis_survivor {
            v.push("Survivor");
        }
        if self.festival_goer {
            v.push("Festival Goer");
        }
        if v.is_empty() {
            "None yet".to_string()
        } else {
            v.join("  ")
        }
    }
}

// ── Goal / Game State ─────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum GoalKind {
    EarnMoney,
    WorkTimes,
    MaintainHappy,
    EatTimes,
    ChatTimes,
    FriendNpc,
    SaveMoney,
    ExerciseTimes,
    LowerStress,
    BuildStreak,
    MasterHobby,
    EarnPassive,
    OutdoorWeather,
    StudyTimes,
    FeedPet,
    ThrowParty,
    OwnVehicle,
    SeasonalGoal,
}

#[derive(Resource)]
pub struct DailyGoal {
    pub kind: GoalKind,
    pub description: String,
    pub progress: f32,
    pub target: f32,
    pub reward_money: f32,
    pub reward_happiness: f32,
    pub completed: bool,
    pub failed: bool,
}
impl Default for DailyGoal {
    fn default() -> Self {
        Self {
            kind: GoalKind::EarnMoney,
            description: "Earn $50 today".to_string(),
            progress: 0.,
            target: 50.,
            reward_money: 25.,
            reward_happiness: 0.,
            completed: false,
            failed: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct GameState {
    pub days_survived: u32,
    pub money_earned_today: f32,
    pub work_today: u32,
    pub eat_today: u32,
    pub chat_today: u32,
    pub exercise_today: u32,
    pub hobby_today: u32,
    pub passive_income_today: f32,
    pub study_today: u32,
    pub outdoor_done_today: bool,
    pub high_stress_today: bool,
    pub high_hunger_today: bool,
    /// Set when the player is evicted; cleared by apply_eviction_teleport after repositioning.
    pub just_evicted: bool,
    /// Tracks the last day best-friend perks were applied.
    pub bf_perk_day: u32,
    /// Lifetime gifts given to NPCs.
    pub total_gifts: u32,
    /// Lifetime items crafted.
    pub total_crafted: u32,
    /// Lifetime quests completed.
    pub total_quests: u32,
}

// ── Mood ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Mood {
    Elated,
    Happy,
    Okay,
    Sad,
    Depressed,
}
impl Mood {
    pub fn from_happiness(h: f32) -> Self {
        match h as u32 {
            80..=100 => Self::Elated,
            60..=79 => Self::Happy,
            40..=59 => Self::Okay,
            20..=39 => Self::Sad,
            _ => Self::Depressed,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Elated => "Elated",
            Self::Happy => "Happy",
            Self::Okay => "Okay",
            Self::Sad => "Sad",
            Self::Depressed => "Depressed",
        }
    }
    pub fn decay_mult(&self) -> f32 {
        match self {
            Self::Elated => 0.6,
            Self::Happy => 0.8,
            Self::Okay => 1.0,
            Self::Sad => 1.2,
            Self::Depressed => 1.5,
        }
    }
    pub fn work_mult(&self) -> f32 {
        match self {
            Self::Elated => 1.2,
            Self::Happy => 1.1,
            Self::Okay => 1.0,
            Self::Sad => 0.9,
            Self::Depressed => 0.75,
        }
    }
}

// ── Weather ───────────────────────────────────────────────────────────────────

#[derive(Resource, Default, Clone, PartialEq)]
pub enum WeatherKind {
    #[default]
    Sunny,
    Cloudy,
    Rainy,
    Stormy,
}
impl WeatherKind {
    pub fn from_day(day: u32) -> Self {
        match day.wrapping_mul(1664525).wrapping_add(1013904223) % 10 {
            0..=3 => Self::Sunny,
            4..=6 => Self::Cloudy,
            7..=8 => Self::Rainy,
            _ => Self::Stormy,
        }
    }
    pub fn energy_decay_mult(&self) -> f32 {
        match self {
            Self::Sunny => 0.9,
            Self::Cloudy => 1.0,
            Self::Rainy => 1.2,
            Self::Stormy => 1.5,
        }
    }
    pub fn outdoor_hap_bonus(&self) -> f32 {
        match self {
            Self::Sunny => 10.,
            Self::Cloudy => 0.,
            Self::Rainy => -5.,
            Self::Stormy => 0.,
        }
    }
    pub fn is_stormy(&self) -> bool {
        *self == Self::Stormy
    }
    pub fn is_sunny(&self) -> bool {
        *self == Self::Sunny
    }
}

// ── Lightning ─────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct LightningTimer {
    pub next_flash: f32,
    pub flash_alpha: f32,
}
impl Default for LightningTimer {
    fn default() -> Self {
        Self {
            next_flash: 8.0,
            flash_alpha: 0.0,
        }
    }
}

// ── Hobbies ───────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct Hobbies {
    pub painting: f32,
    pub gaming: f32,
    pub music: f32,
}
impl Hobbies {
    pub fn passive_income(&self) -> f32 {
        let mut total = 0.;
        if self.painting >= 3. {
            total += 6.;
        }
        if self.gaming >= 3. {
            total += 4.;
        }
        if self.music >= 3. {
            total += 8.;
        }
        total
    }
    pub fn best(&self) -> f32 {
        self.painting.max(self.gaming).max(self.music)
    }
}

// ── Health Conditions ─────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct Conditions {
    pub burnout: bool,
    pub burnout_days: u32,
    pub malnourished: bool,
    pub malnourish_days: u32,
    /// Player was hospitalised (health reached 0). Blocks all interactions.
    pub hospitalized: bool,
    /// In-game hours remaining until hospital discharge.
    pub hospital_timer: f32,
    /// Chronic stress: stress >75 for 3+ consecutive days.
    pub mental_fatigue: bool,
    pub high_stress_days: u32,
    /// Days stress has been below 45 (used to clear mental fatigue).
    pub low_stress_days: u32,
}
impl Conditions {
    pub fn work_pay_mult(&self) -> f32 {
        let mut mult = 1.0_f32;
        if self.burnout {
            mult *= 0.70;
        }
        if self.malnourished {
            mult *= 0.85;
        }
        if self.mental_fatigue {
            mult *= 0.85;
        }
        mult
    }
}

// ── Crisis Events ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CrisisKind {
    Layoff,
    MarketCrash,
    MedicalEmergency,
    RentHike,
    Theft,
    ApplianceBreak,
}
impl From<&CrisisKind> for u8 {
    fn from(k: &CrisisKind) -> u8 {
        match k {
            CrisisKind::Layoff => 1,
            CrisisKind::MarketCrash => 2,
            CrisisKind::MedicalEmergency => 3,
            CrisisKind::RentHike => 4,
            CrisisKind::Theft => 5,
            CrisisKind::ApplianceBreak => 6,
        }
    }
}
impl CrisisKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Layoff),
            2 => Some(Self::MarketCrash),
            3 => Some(Self::MedicalEmergency),
            4 => Some(Self::RentHike),
            5 => Some(Self::Theft),
            6 => Some(Self::ApplianceBreak),
            _ => None,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Layoff => "Layoff",
            Self::MarketCrash => "Market Crash",
            Self::MedicalEmergency => "Medical Emergency",
            Self::RentHike => "Rent Hike",
            Self::Theft => "Theft",
            Self::ApplianceBreak => "Appliance Breakdown",
        }
    }
    pub fn duration(&self) -> u32 {
        match self {
            Self::Layoff => 3,
            Self::MarketCrash => 1,
            Self::MedicalEmergency => 2,
            Self::RentHike => 5,
            Self::Theft => 1,
            Self::ApplianceBreak => 3,
        }
    }
}

#[derive(Resource, Default)]
pub struct CrisisState {
    /// Currently active crisis, if any.
    pub active: Option<CrisisKind>,
    /// Days remaining in the active crisis.
    pub days_left: u32,
    /// Total crises survived (lifetime).
    pub crises_survived: u32,
    /// Day the last crisis ended (cooldown tracking).
    pub last_crisis_day: u32,
    /// Whether the player has insurance (bought at the bank).
    pub has_insurance: bool,
    /// Days of insurance remaining.
    pub insurance_days: u32,
    /// Last day the crisis system ran (sentinel to run once per day).
    pub last_check_day: u32,
}
impl CrisisState {
    pub fn is_active(&self) -> bool {
        self.active.is_some() && self.days_left > 0
    }
    pub fn is_laid_off(&self) -> bool {
        matches!(self.active, Some(CrisisKind::Layoff)) && self.days_left > 0
    }
    pub fn rent_multiplier(&self) -> f32 {
        if matches!(self.active, Some(CrisisKind::RentHike)) && self.days_left > 0 {
            2.0
        } else {
            1.0
        }
    }
    pub fn home_cost_extra(&self) -> f32 {
        if matches!(self.active, Some(CrisisKind::ApplianceBreak)) && self.days_left > 0 {
            8.0
        } else {
            0.0
        }
    }
}

// ── Investment ────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct Investment {
    pub amount: f32,
    pub risk: u8,
    pub daily_return_rate: f32,
    pub total_return: f32,
}

// ── Reputation ────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct Reputation {
    pub score: f32,
}
impl Reputation {
    pub fn add_score(&mut self, amount: f32) {
        self.score = (self.score + amount).clamp(0., 100.);
    }
    pub fn chat_bonus(&self) -> f32 {
        1.0 + self.score * 0.005
    }
    pub fn work_mult(&self) -> f32 {
        if self.score < 20. {
            0.90
        } else if self.score >= 60. {
            1.10
        } else {
            1.00
        }
    }
}

// ── Transport ─────────────────────────────────────────────────────────────────

#[derive(Default, Clone, PartialEq)]
pub enum TransportKind {
    #[default]
    Walk,
    Bike,
    Car,
}
impl From<&TransportKind> for u8 {
    fn from(t: &TransportKind) -> u8 {
        match t {
            TransportKind::Walk => 0,
            TransportKind::Bike => 1,
            TransportKind::Car => 2,
        }
    }
}
impl From<u8> for TransportKind {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::Bike,
            2 => Self::Car,
            _ => Self::Walk,
        }
    }
}
impl TransportKind {
    pub fn work_bonus(&self) -> f32 {
        match self {
            Self::Walk => 1.0,
            Self::Bike => 1.10,
            Self::Car => 1.20,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Walk => "On foot",
            Self::Bike => "Bicycle 🚲",
            Self::Car => "Car 🚗",
        }
    }
    pub fn is_vehicle(&self) -> bool {
        *self != Self::Walk
    }
}

#[derive(Resource, Default)]
pub struct Transport {
    pub kind: TransportKind,
    pub work_uses: u32,
    pub maintenance_due: bool,
}
impl Transport {
    pub fn effective_work_bonus(&self) -> f32 {
        if self.maintenance_due {
            1.0
        } else {
            self.kind.work_bonus()
        }
    }
}

// ── Pet ───────────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct Pet {
    pub has_pet: bool,
    pub hunger: f32,
    pub fed_today: bool,
    pub name: String,
    pub kind: PetKind,
}
impl Default for Pet {
    fn default() -> Self {
        Self {
            has_pet: false,
            hunger: 0.,
            fed_today: false,
            name: "Buddy".to_string(),
            kind: PetKind::Dog,
        }
    }
}

// ── VehicleState ──────────────────────────────────────────────────────────────

#[derive(Component, Default)]
pub struct VehicleState {
    pub in_vehicle: bool,
}

// ── Social Events ─────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct SocialEvents {
    pub parties_thrown: u32,
    pub party_today: bool,
}

// ── Season ────────────────────────────────────────────────────────────────────

#[derive(Default, Clone, PartialEq)]
pub enum SeasonKind {
    #[default]
    Spring,
    Summer,
    Autumn,
    Winter,
}
impl SeasonKind {
    pub fn from_day(day: u32) -> Self {
        match (day / 30) % 4 {
            0 => Self::Spring,
            1 => Self::Summer,
            2 => Self::Autumn,
            _ => Self::Winter,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Spring => "🌸 Spring",
            Self::Summer => "☀ Summer",
            Self::Autumn => "🍂 Autumn",
            Self::Winter => "❄ Winter",
        }
    }
    pub fn social_mult(&self) -> f32 {
        match self {
            Self::Spring => 1.25,
            _ => 1.0,
        }
    }
    pub fn outdoor_bonus(&self) -> f32 {
        match self {
            Self::Summer => 8.,
            _ => 0.,
        }
    }
    pub fn passive_mult(&self) -> f32 {
        match self {
            Self::Autumn => 1.25,
            _ => 1.0,
        }
    }
    pub fn indoor_bonus(&self) -> f32 {
        match self {
            Self::Winter => 5.,
            _ => 0.,
        }
    }
    pub fn seasonal_goal_desc(&self) -> (&'static str, f32) {
        match self {
            Self::Spring => ("Exercise twice today (Spring energy!)", 2.),
            Self::Summer => ("Go outside today (Summer sunshine!)", 1.),
            Self::Autumn => ("Earn $5 passive income (Autumn harvest!)", 5.),
            Self::Winter => ("Do 2 hobbies indoors (Winter coziness!)", 2.),
        }
    }
}

#[derive(Resource, Default)]
pub struct Season {
    pub current: SeasonKind,
}

// ── Bank Input Dialog ─────────────────────────────────────────────────────────

#[derive(Default, PartialEq, Clone, Copy)]
pub enum BankInputKind {
    #[default]
    Deposit,
    Withdraw,
    InvestLow,
    InvestMedium,
}

#[derive(Component, Default)]
pub struct BankInput {
    pub active: bool,
    pub buffer: String,
    pub kind: BankInputKind,
}

#[derive(Clone, PartialEq)]
pub enum PendingAction {
    Action(ActionKind),
    Gift,
    Bank(u8),
    Transport(u8),
    Craft(u8),
    Festival(u8),
}

#[derive(Component, Default)]
pub struct ActionPrompt {
    pub active: bool,
    pub buffer: String,
    pub label: String,
    pub instruction: String,
    pub expected: String,
    pub retries_left: u8,
    pub pending: Option<PendingAction>,
    pub target: Option<Entity>,
}
impl ActionPrompt {
    pub fn clear(&mut self) {
        self.active = false;
        self.buffer.clear();
        self.label.clear();
        self.instruction.clear();
        self.expected.clear();
        self.retries_left = 0;
        self.pending = None;
        self.target = None;
    }

    #[allow(dead_code)]
    pub fn display_text(&self) -> String {
        String::new() // typing overlay handles display
    }
}

// ── Festivals ─────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub enum FestivalKind {
    SpringFair,
    SummerBBQ,
    AutumnHarvest,
    WinterGala,
}
impl FestivalKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SpringFair => "Spring Fair",
            Self::SummerBBQ => "Summer BBQ",
            Self::AutumnHarvest => "Autumn Harvest",
            Self::WinterGala => "Winter Gala",
        }
    }
}

#[derive(Resource, Default)]
pub struct FestivalState {
    pub active: Option<FestivalKind>,
    pub tokens: u32,
    pub activities_today: u8,
    pub spring_attended: bool,
    pub summer_attended: bool,
    pub autumn_attended: bool,
    pub winter_attended: bool,
    pub festivals_total: u32,
    pub last_check_day: u32,
}
impl FestivalState {
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }
    pub fn all_seasons_attended(&self) -> bool {
        self.spring_attended && self.summer_attended && self.autumn_attended && self.winter_attended
    }
}

// ── Tutorial ──────────────────────────────────────────────────────────────────

/// Text shown on each tutorial step. Index 0 = first slide shown on new game.
pub const TUTORIAL_STEPS: &[(&str, &str)] = &[
    (
        "Welcome to Everyday Life Simulator",
        "Use WASD or arrow keys to move.\nSurvive day-to-day by managing energy, hunger, and money.",
    ),
    (
        "Your Stats",
        "Energy, Hunger, Happiness, Health, and Stress are shown on the left.\nKeep them balanced - low energy or hunger will hurt you.",
    ),
    (
        "Working and Earning",
        "Walk to the OFFICE and press [E] to work.\nWork pays your rent and fills your savings.\nYour career skill grows with each shift.",
    ),
    (
        "Eating and Sleeping",
        "Buy food at the SHOP, then use it from the APARTMENT.\nSleep at your apartment to restore energy.\nHigh sleep debt tanks your health.",
    ),
    (
        "Socialising",
        "Talk to NPCs with [E] to build friendship.\nGift items with [G]. Hang out at friendship level 3+ with [H].\nFriends provide bonuses and quests.",
    ),
    (
        "Key Bindings",
        "[E] Interact   [G] Gift   [H] Hangout\n[B] Bank input   [Tab] Skills panel\n[F5] Save   [Esc] Cancel prompt\n\nGood luck!",
    ),
];

/// Tracks which tutorial step is currently showing.
/// `step == 0` means the tutorial is inactive (hidden).
/// Steps 1..=TUTORIAL_STEPS.len() are the active slides (1-based so default=0 = off).
#[derive(Resource, Default)]
pub struct TutorialState {
    pub step: usize,
}

impl TutorialState {
    pub fn is_active(&self) -> bool {
        self.step > 0 && self.step <= TUTORIAL_STEPS.len()
    }
    pub fn current(&self) -> Option<(&'static str, &'static str)> {
        TUTORIAL_STEPS.get(self.step.saturating_sub(1)).copied()
    }
    pub fn advance(&mut self) {
        if self.step <= TUTORIAL_STEPS.len() {
            self.step += 1;
        }
    }
    pub fn dismiss(&mut self) {
        self.step = TUTORIAL_STEPS.len() + 1;
    }
}

// ── SystemParam Groups (keeps update_hud and handle_interaction under 16 params) ──

#[derive(SystemParam)]
pub struct HudExtras<'w, 's> {
    pub weather: Res<'w, WeatherKind>,
    pub hobbies: Res<'w, Hobbies>,
    pub conds: Res<'w, Conditions>,
    pub rep: Res<'w, Reputation>,
    pub pet: Res<'w, Pet>,
    pub transport: Res<'w, Transport>,
    pub season: Res<'w, Season>,
    pub settings: Res<'w, GameSettings>,
    pub story: Res<'w, NarrativeState>,
    pub player_vehicle_q: Query<'w, 's, &'static VehicleState, With<LocalPlayer>>,
    pub player_prompt_q: Query<'w, 's, &'static ActionPrompt, With<LocalPlayer>>,
    pub quest_board: Res<'w, QuestBoard>,
    pub crisis: Res<'w, CrisisState>,
    pub festival: Res<'w, FestivalState>,
}

#[derive(SystemParam)]
pub struct InteractExtras<'w> {
    pub hobbies: ResMut<'w, Hobbies>,
    pub weather: Res<'w, WeatherKind>,
    pub rep: ResMut<'w, Reputation>,
    pub invest: ResMut<'w, Investment>,
    pub conds: Res<'w, Conditions>,
    pub pet: ResMut<'w, Pet>,
    pub transport: ResMut<'w, Transport>,
    pub social_events: ResMut<'w, SocialEvents>,
    pub season: Res<'w, Season>,
    pub settings: Res<'w, GameSettings>,
    pub quest_board: ResMut<'w, QuestBoard>,
    pub crisis: ResMut<'w, CrisisState>,
    pub festival: ResMut<'w, FestivalState>,
}

#[derive(SystemParam)]
pub struct DayExtras<'w> {
    pub pet: ResMut<'w, Pet>,
    pub social_events: ResMut<'w, SocialEvents>,
    pub season: ResMut<'w, Season>,
    pub save_writer: EventWriter<'w, SaveRequest>,
    pub settings: Res<'w, GameSettings>,
    pub quest_board: ResMut<'w, QuestBoard>,
    pub crisis: Res<'w, CrisisState>,
}

#[derive(SystemParam)]
pub struct MilestoneExtras<'w> {
    pub crisis: Res<'w, CrisisState>,
    pub festival: Res<'w, FestivalState>,
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn career_bonus_scales_with_level() {
        let mut s = Skills::default();
        assert!((s.career_bonus() - 1.0).abs() < f32::EPSILON);
        s.career = 5.0;
        assert!((s.career_bonus() - 1.60).abs() < 0.001);
    }

    #[test]
    fn fitness_bonus_scales_with_level() {
        let mut s = Skills::default();
        assert!((s.fitness_bonus() - 1.0).abs() < f32::EPSILON);
        s.fitness = 5.0;
        assert!((s.fitness_bonus() - 1.25).abs() < 0.001);
    }

    #[test]
    fn mood_decay_mult_ordering() {
        assert!(Mood::Depressed.decay_mult() > Mood::Sad.decay_mult());
        assert!(Mood::Sad.decay_mult() > Mood::Okay.decay_mult());
        assert!(Mood::Okay.decay_mult() > Mood::Happy.decay_mult());
        assert!(Mood::Happy.decay_mult() > Mood::Elated.decay_mult());
    }

    #[test]
    fn mood_from_happiness_boundaries() {
        assert!(matches!(Mood::from_happiness(100.), Mood::Elated));
        assert!(matches!(Mood::from_happiness(0.), Mood::Depressed));
    }

    #[test]
    fn conditions_work_pay_mult_burnout() {
        let mut c = Conditions::default();
        assert!((c.work_pay_mult() - 1.0).abs() < f32::EPSILON);
        c.burnout = true;
        assert!((c.work_pay_mult() - 0.70).abs() < 0.001);
    }

    #[test]
    fn milestones_count_increments() {
        let mut ms = Milestones::default();
        assert_eq!(ms.count(), 0);
        ms.exec = true;
        ms.hobbyist = true;
        assert_eq!(ms.count(), 2);
    }

    #[test]
    fn milestones_summary_none() {
        let ms = Milestones::default();
        assert_eq!(ms.summary(), "None yet");
    }

    #[test]
    fn narrative_unlocks_only_once() {
        let mut story = NarrativeState::default();
        assert!(story.unlock(
            "intro",
            "New in Town",
            "The city feels bigger than your apartment."
        ));
        assert!(!story.unlock(
            "intro",
            "New in Town",
            "The city feels bigger than your apartment."
        ));
        assert_eq!(story.count(), 1);
    }

    #[test]
    fn narrative_summary_uses_latest_entry() {
        let mut story = NarrativeState::default();
        story.unlock(
            "career",
            "A Door Opens",
            "Your work is finally being noticed.",
        );
        assert!(story.latest_summary().contains("A Door Opens"));
        assert!(story.latest_summary().contains("noticed"));
    }

    // ── PlayerStats multipliers ───────────────────────────────────────────────

    #[test]
    fn stress_work_mult_tiers() {
        let mut s = PlayerStats::default();
        s.stress = 40.;
        assert!((s.stress_work_mult() - 1.0).abs() < f32::EPSILON);
        s.stress = 60.;
        assert!((s.stress_work_mult() - 0.85).abs() < f32::EPSILON);
        s.stress = 80.;
        assert!((s.stress_work_mult() - 0.50).abs() < f32::EPSILON);
    }

    #[test]
    fn stress_work_mult_boundaries() {
        let mut s = PlayerStats::default();
        s.stress = 50.;
        assert!(
            (s.stress_work_mult() - 1.0).abs() < f32::EPSILON,
            "50 is not >50, so tier 1"
        );
        s.stress = 50.1;
        assert!(
            (s.stress_work_mult() - 0.85).abs() < f32::EPSILON,
            "50.1 is >50, tier 2"
        );
        s.stress = 75.;
        assert!(
            (s.stress_work_mult() - 0.85).abs() < f32::EPSILON,
            "75 is not >75, still tier 2"
        );
        s.stress = 75.1;
        assert!(
            (s.stress_work_mult() - 0.50).abs() < f32::EPSILON,
            "75.1 is >75, tier 3"
        );
    }

    #[test]
    fn loan_penalty_tiers() {
        let mut s = PlayerStats::default();
        s.loan = 300.;
        assert!(
            (s.loan_penalty() - 1.0).abs() < f32::EPSILON,
            "300 is not >300"
        );
        s.loan = 300.1;
        assert!(
            (s.loan_penalty() - 0.90).abs() < f32::EPSILON,
            "300.1 is >300"
        );
        s.loan = 0.;
        assert!((s.loan_penalty() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn skill_gain_mult_tiers() {
        let mut s = PlayerStats::default();
        s.sleep_debt = 0.;
        assert!((s.skill_gain_mult() - 1.0).abs() < f32::EPSILON);
        s.sleep_debt = 10.;
        assert!((s.skill_gain_mult() - 0.80).abs() < f32::EPSILON);
        s.sleep_debt = 20.;
        assert!((s.skill_gain_mult() - 0.60).abs() < f32::EPSILON);
    }

    // ── Reputation ────────────────────────────────────────────────────────────

    #[test]
    fn reputation_work_mult_tiers() {
        let mut r = Reputation::default();
        r.score = 10.;
        assert!(
            (r.work_mult() - 0.90).abs() < f32::EPSILON,
            "score <20 → penalty"
        );
        r.score = 40.;
        assert!(
            (r.work_mult() - 1.00).abs() < f32::EPSILON,
            "score 20-59 → neutral"
        );
        r.score = 60.;
        assert!(
            (r.work_mult() - 1.10).abs() < f32::EPSILON,
            "score >=60 → bonus"
        );
        r.score = 100.;
        assert!((r.work_mult() - 1.10).abs() < f32::EPSILON);
    }

    #[test]
    fn reputation_work_mult_boundary_at_20_and_60() {
        let mut r = Reputation::default();
        r.score = 19.9;
        assert!((r.work_mult() - 0.90).abs() < f32::EPSILON);
        r.score = 20.;
        assert!((r.work_mult() - 1.00).abs() < f32::EPSILON);
        r.score = 59.9;
        assert!((r.work_mult() - 1.00).abs() < f32::EPSILON);
        r.score = 60.;
        assert!((r.work_mult() - 1.10).abs() < f32::EPSILON);
    }

    #[test]
    fn reputation_chat_bonus_scales_linearly() {
        let mut r = Reputation::default();
        r.score = 0.;
        assert!((r.chat_bonus() - 1.0).abs() < f32::EPSILON);
        r.score = 100.;
        assert!((r.chat_bonus() - 1.5).abs() < f32::EPSILON);
        r.score = 50.;
        assert!((r.chat_bonus() - 1.25).abs() < f32::EPSILON);
    }

    // ── Skills::work_pay ─────────────────────────────────────────────────────

    #[test]
    fn work_pay_career_tiers_no_streak() {
        let mut s = Skills::default();
        // Junior (career=0): base=30, career_bonus=1.0, streak=1.0 → 30.0
        assert!((s.work_pay(0) - 30.).abs() < 0.01);
        // Senior (career=2.5): base=45, career_bonus=1.30, streak=1.0 → 58.5
        s.career = 2.5;
        assert!((s.work_pay(0) - 58.5).abs() < 0.01);
        // Executive (career=5.0): base=70, career_bonus=1.60 → 112.0
        s.career = 5.0;
        assert!((s.work_pay(0) - 112.).abs() < 0.01);
    }

    #[test]
    fn work_pay_streak_multipliers() {
        let s = Skills::default(); // career=0, base=30, bonus=1.0
        assert!((s.work_pay(0) - 30.0).abs() < 0.01, "no streak");
        assert!((s.work_pay(3) - 33.0).abs() < 0.01, "streak 3 → 1.10x");
        assert!((s.work_pay(5) - 36.0).abs() < 0.01, "streak 5 → 1.20x");
        assert!((s.work_pay(7) - 40.5).abs() < 0.01, "streak 7 → 1.35x");
    }

    #[test]
    fn work_pay_streak_boundary_at_3_5_7() {
        let s = Skills::default();
        assert!((s.work_pay(2) - 30.0).abs() < 0.01, "streak 2 → no bonus");
        assert!((s.work_pay(3) - 33.0).abs() < 0.01, "streak 3 → 1.10x");
        assert!(
            (s.work_pay(4) - 33.0).abs() < 0.01,
            "streak 4 → still 1.10x"
        );
        assert!((s.work_pay(5) - 36.0).abs() < 0.01, "streak 5 → 1.20x");
        assert!(
            (s.work_pay(6) - 36.0).abs() < 0.01,
            "streak 6 → still 1.20x"
        );
        assert!((s.work_pay(7) - 40.5).abs() < 0.01, "streak 7 → 1.35x");
    }

    // ── Skills misc ───────────────────────────────────────────────────────────

    #[test]
    fn cooking_and_social_bonus_scale() {
        let mut s = Skills::default();
        assert!((s.cooking_bonus() - 1.0).abs() < f32::EPSILON);
        s.cooking = 5.0;
        assert!((s.cooking_bonus() - 1.5).abs() < 0.001);
        assert!((s.social_bonus() - 1.0).abs() < f32::EPSILON);
        s.social = 5.0;
        assert!((s.social_bonus() - 1.5).abs() < 0.001);
    }

    #[test]
    fn career_rank_tiers() {
        let mut s = Skills::default();
        assert_eq!(s.career_rank(), "Junior");
        s.career = 2.5;
        assert_eq!(s.career_rank(), "Senior");
        s.career = 5.0;
        assert_eq!(s.career_rank(), "Executive");
    }

    // ── HousingTier ──────────────────────────────────────────────────────────

    #[test]
    fn housing_has_access() {
        assert!(!HousingTier::Unhoused.has_access());
        assert!(HousingTier::Apartment.has_access());
        assert!(HousingTier::Condo.has_access());
        assert!(HousingTier::Penthouse.has_access());
    }

    #[test]
    fn housing_upgrade_cost_chain() {
        assert!((HousingTier::Unhoused.upgrade_cost().unwrap() - 90.).abs() < f32::EPSILON);
        assert!((HousingTier::Apartment.upgrade_cost().unwrap() - 350.).abs() < f32::EPSILON);
        assert!((HousingTier::Condo.upgrade_cost().unwrap() - 800.).abs() < f32::EPSILON);
        assert!(HousingTier::Penthouse.upgrade_cost().is_none());
    }

    #[test]
    fn housing_next_chain() {
        assert!(matches!(
            HousingTier::Unhoused.next(),
            Some(HousingTier::Apartment)
        ));
        assert!(matches!(
            HousingTier::Apartment.next(),
            Some(HousingTier::Condo)
        ));
        assert!(matches!(
            HousingTier::Condo.next(),
            Some(HousingTier::Penthouse)
        ));
        assert!(HousingTier::Penthouse.next().is_none());
    }

    #[test]
    fn housing_rent_values() {
        assert!((HousingTier::Unhoused.rent() - 0.).abs() < f32::EPSILON);
        assert!((HousingTier::Apartment.rent() - 30.).abs() < f32::EPSILON);
        assert!((HousingTier::Condo.rent() - 50.).abs() < f32::EPSILON);
        assert!((HousingTier::Penthouse.rent() - 85.).abs() < f32::EPSILON);
    }

    // ── Conditions::work_pay_mult stacking ───────────────────────────────────

    #[test]
    fn conditions_work_pay_mult_mental_fatigue() {
        let mut c = Conditions::default();
        c.mental_fatigue = true;
        assert!((c.work_pay_mult() - 0.85).abs() < 0.001);
    }

    #[test]
    fn conditions_work_pay_mult_malnourished() {
        let mut c = Conditions::default();
        c.malnourished = true;
        assert!((c.work_pay_mult() - 0.85).abs() < 0.001);
    }

    #[test]
    fn conditions_work_pay_mult_all_stack_multiplicatively() {
        let mut c = Conditions::default();
        c.burnout = true;
        c.malnourished = true;
        c.mental_fatigue = true;
        // 0.70 * 0.85 * 0.85 ≈ 0.50575
        let expected = 0.70_f32 * 0.85 * 0.85;
        assert!((c.work_pay_mult() - expected).abs() < 0.001);
    }

    #[test]
    fn conditions_work_pay_mult_burnout_and_fatigue() {
        let mut c = Conditions::default();
        c.burnout = true;
        c.mental_fatigue = true;
        let expected = 0.70_f32 * 0.85;
        assert!((c.work_pay_mult() - expected).abs() < 0.001);
    }

    // ── Notification ──────────────────────────────────────────────────────────

    #[test]
    fn notification_default_is_empty() {
        let n = Notification::default();
        assert!(n.message.is_empty());
        assert!((n.timer - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn notification_push_displays_immediately_when_idle() {
        let mut n = Notification::default();
        n.push("hello", 3.0);
        assert_eq!(n.message, "hello");
        assert!((n.timer - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn notification_push_queues_when_busy() {
        let mut n = Notification::default();
        n.push("first", 5.0);
        n.push("second", 2.0);
        // First is shown; second is in the queue.
        assert_eq!(n.message, "first");
        // Tick past the first message to surface the queued one.
        n.tick(5.0);
        assert_eq!(n.message, "second");
        assert!((n.timer - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn notification_queue_caps_at_four() {
        let mut n = Notification::default();
        n.push("displayed", 1.0);
        for i in 0..6 {
            n.push(format!("queued-{i}"), 1.0);
        }
        // Drain and count: first display + 4 queued = 5 messages survive,
        // the last 2 pushes are dropped on the floor.
        let mut seen = vec![n.message.clone()];
        for _ in 0..6 {
            n.tick(1.0);
            if !n.message.is_empty() && !seen.contains(&n.message) {
                seen.push(n.message.clone());
            }
        }
        assert_eq!(seen.len(), 5, "got {:?}", seen);
        assert_eq!(seen[0], "displayed");
        assert_eq!(seen[1], "queued-0");
        assert_eq!(seen[4], "queued-3");
    }

    #[test]
    fn notification_tick_does_nothing_when_idle() {
        let mut n = Notification::default();
        n.tick(1.0);
        assert!(n.message.is_empty());
        assert!((n.timer - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn notification_tick_decrements_timer() {
        let mut n = Notification::default();
        n.push("msg", 5.0);
        n.tick(1.5);
        assert!((n.timer - 3.5).abs() < 0.0001);
        assert_eq!(n.message, "msg");
    }

    #[test]
    fn notification_flush_message_pushes_through_queue() {
        let mut n = Notification::default();
        // Direct write (as some systems do), then flush.
        n.message = "direct".into();
        n.flush_message(2.0);
        assert_eq!(n.message, "direct");
        assert!((n.timer - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn notification_flush_message_no_op_on_empty() {
        let mut n = Notification::default();
        n.flush_message(2.0);
        assert!(n.message.is_empty());
        assert!((n.timer - 0.0).abs() < f32::EPSILON);
    }

    // ── FestivalKind / FestivalState ──────────────────────────────────────────

    #[test]
    fn festival_kind_label_per_season() {
        assert_eq!(FestivalKind::SpringFair.label(), "Spring Fair");
        assert_eq!(FestivalKind::SummerBBQ.label(), "Summer BBQ");
        assert_eq!(FestivalKind::AutumnHarvest.label(), "Autumn Harvest");
        assert_eq!(FestivalKind::WinterGala.label(), "Winter Gala");
    }

    #[test]
    fn festival_state_is_active_reflects_active_field() {
        let mut f = FestivalState::default();
        assert!(!f.is_active());
        f.active = Some(FestivalKind::SpringFair);
        assert!(f.is_active());
        f.active = None;
        assert!(!f.is_active());
    }

    #[test]
    fn festival_state_all_seasons_attended_requires_all_four() {
        let mut f = FestivalState::default();
        assert!(!f.all_seasons_attended());
        f.spring_attended = true;
        f.summer_attended = true;
        f.autumn_attended = true;
        assert!(!f.all_seasons_attended(), "missing winter");
        f.winter_attended = true;
        assert!(f.all_seasons_attended());
    }

    // ── QuestBoard ────────────────────────────────────────────────────────────

    fn make_quest(npc_id: usize, completed: bool) -> NpcQuest {
        NpcQuest {
            npc_id,
            kind: crate::components::QuestKind::EarnMoney(60.),
            description: "test".into(),
            reward_money: 10.,
            reward_friendship: 0.5,
            progress: 0,
            target: 1,
            completed,
        }
    }

    #[test]
    fn quest_board_active_count_excludes_completed() {
        let mut qb = QuestBoard::default();
        assert_eq!(qb.active_count(), 0);
        qb.quests.push(make_quest(0, false));
        qb.quests.push(make_quest(1, true));
        qb.quests.push(make_quest(2, false));
        assert_eq!(qb.active_count(), 2);
    }

    #[test]
    fn quest_board_has_quest_from_only_matches_active() {
        let mut qb = QuestBoard::default();
        assert!(!qb.has_quest_from(0));
        qb.quests.push(make_quest(0, false));
        assert!(qb.has_quest_from(0));
        assert!(!qb.has_quest_from(1));
        // Completed quests for an NPC do not block giving a new one.
        qb.quests.clear();
        qb.quests.push(make_quest(0, true));
        assert!(!qb.has_quest_from(0));
    }

    // ── SeasonKind ────────────────────────────────────────────────────────────

    #[test]
    fn season_from_day_cycles_every_120_days() {
        assert!(matches!(SeasonKind::from_day(0), SeasonKind::Spring));
        assert!(matches!(SeasonKind::from_day(29), SeasonKind::Spring));
        assert!(matches!(SeasonKind::from_day(30), SeasonKind::Summer));
        assert!(matches!(SeasonKind::from_day(60), SeasonKind::Autumn));
        assert!(matches!(SeasonKind::from_day(90), SeasonKind::Winter));
        // Wraps after 120 days.
        assert!(matches!(SeasonKind::from_day(120), SeasonKind::Spring));
        assert!(matches!(SeasonKind::from_day(150), SeasonKind::Summer));
    }

    #[test]
    fn season_label_includes_emoji_and_name() {
        // The label is the exact emoji-prefixed string shown in HUD.
        assert_eq!(SeasonKind::Spring.label(), "🌸 Spring");
        assert_eq!(SeasonKind::Summer.label(), "☀ Summer");
        assert_eq!(SeasonKind::Autumn.label(), "🍂 Autumn");
        assert_eq!(SeasonKind::Winter.label(), "❄ Winter");
    }

    #[test]
    fn season_modifiers_only_affect_their_own_season() {
        // social_mult: only Spring gets a bonus.
        assert!((SeasonKind::Spring.social_mult() - 1.25).abs() < 0.001);
        for s in [SeasonKind::Summer, SeasonKind::Autumn, SeasonKind::Winter] {
            assert!((s.social_mult() - 1.0).abs() < f32::EPSILON);
        }
        // outdoor_bonus: only Summer.
        assert!((SeasonKind::Summer.outdoor_bonus() - 8.0).abs() < f32::EPSILON);
        for s in [SeasonKind::Spring, SeasonKind::Autumn, SeasonKind::Winter] {
            assert!((s.outdoor_bonus() - 0.0).abs() < f32::EPSILON);
        }
        // passive_mult: only Autumn.
        assert!((SeasonKind::Autumn.passive_mult() - 1.25).abs() < 0.001);
        for s in [SeasonKind::Spring, SeasonKind::Summer, SeasonKind::Winter] {
            assert!((s.passive_mult() - 1.0).abs() < f32::EPSILON);
        }
        // indoor_bonus: only Winter.
        assert!((SeasonKind::Winter.indoor_bonus() - 5.0).abs() < f32::EPSILON);
        for s in [SeasonKind::Spring, SeasonKind::Summer, SeasonKind::Autumn] {
            assert!((s.indoor_bonus() - 0.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn season_seasonal_goal_targets_match_descriptions() {
        // Each season has a distinct goal description and a positive target.
        let mut descs = Vec::new();
        for s in [
            SeasonKind::Spring,
            SeasonKind::Summer,
            SeasonKind::Autumn,
            SeasonKind::Winter,
        ] {
            let (desc, target) = s.seasonal_goal_desc();
            assert!(!desc.is_empty(), "{} missing desc", s.label());
            assert!(target > 0.0, "{} target should be positive", s.label());
            descs.push(desc);
        }
        // No two seasons should share the same description.
        for i in 0..descs.len() {
            for j in (i + 1)..descs.len() {
                assert_ne!(descs[i], descs[j]);
            }
        }
    }

    // ── PlayerStats bounded mutators ──────────────────────────────────────────

    #[test]
    fn modify_health_clamps_to_0_100() {
        let mut s = PlayerStats::default();
        s.health = 50.;
        s.modify_health(200.);
        assert!((s.health - 100.).abs() < f32::EPSILON);
        s.modify_health(-500.);
        assert!((s.health - 0.).abs() < f32::EPSILON);
    }

    #[test]
    fn modify_energy_clamps_to_max_energy() {
        let mut s = PlayerStats::default();
        s.energy = 0.;
        s.sleep_debt = 0.; // max_energy = 100
        s.modify_energy(500.);
        assert!((s.energy - 100.).abs() < f32::EPSILON);
        // Severe sleep debt lowers the cap.
        s.sleep_debt = 20.; // max_energy = 60
        s.modify_energy(500.);
        assert!((s.energy - 60.).abs() < f32::EPSILON);
        s.modify_energy(-1000.);
        assert!((s.energy - 0.).abs() < f32::EPSILON);
    }

    #[test]
    fn modify_happiness_stress_hunger_clamp_to_0_100() {
        let mut s = PlayerStats::default();
        s.happiness = 50.;
        s.modify_happiness(75.);
        assert!((s.happiness - 100.).abs() < f32::EPSILON);
        s.modify_happiness(-500.);
        assert!((s.happiness - 0.).abs() < f32::EPSILON);

        s.stress = 50.;
        s.modify_stress(75.);
        assert!((s.stress - 100.).abs() < f32::EPSILON);
        s.modify_stress(-500.);
        assert!((s.stress - 0.).abs() < f32::EPSILON);

        s.hunger = 50.;
        s.modify_hunger(75.);
        assert!((s.hunger - 100.).abs() < f32::EPSILON);
        s.modify_hunger(-500.);
        assert!((s.hunger - 0.).abs() < f32::EPSILON);
    }

    #[test]
    fn can_afford_uses_debt_limit_credit() {
        let mut s = PlayerStats::default();
        s.money = 10.;
        // money + DEBT_LIMIT must cover cost.
        assert!(s.can_afford(10. + DEBT_LIMIT));
        assert!(!s.can_afford(10. + DEBT_LIMIT + 0.01));
    }

    // ── GameTime helpers ──────────────────────────────────────────────────────

    #[test]
    fn game_time_day_name_cycles_through_week() {
        let mut gt = GameTime::default();
        let names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        for (i, expected) in names.iter().enumerate() {
            gt.day = i as u32;
            assert_eq!(gt.day_name(), *expected);
        }
        // Wraps after 7 days.
        gt.day = 7;
        assert_eq!(gt.day_name(), "Mon");
    }

    // ── Mood::label ───────────────────────────────────────────────────────────

    #[test]
    fn mood_label_for_all_variants() {
        assert_eq!(Mood::Elated.label(), "Elated");
        assert_eq!(Mood::Happy.label(), "Happy");
        assert_eq!(Mood::Okay.label(), "Okay");
        assert_eq!(Mood::Sad.label(), "Sad");
        assert_eq!(Mood::Depressed.label(), "Depressed");
    }

    #[test]
    fn mood_from_happiness_buckets() {
        assert!(matches!(Mood::from_happiness(95.), Mood::Elated));
        assert!(matches!(Mood::from_happiness(80.), Mood::Elated));
        assert!(matches!(Mood::from_happiness(79.), Mood::Happy));
        assert!(matches!(Mood::from_happiness(60.), Mood::Happy));
        assert!(matches!(Mood::from_happiness(59.), Mood::Okay));
        assert!(matches!(Mood::from_happiness(40.), Mood::Okay));
        assert!(matches!(Mood::from_happiness(39.), Mood::Sad));
        assert!(matches!(Mood::from_happiness(20.), Mood::Sad));
        assert!(matches!(Mood::from_happiness(19.), Mood::Depressed));
    }
}
