#![allow(clippy::type_complexity)]

//! Persistence layer: save game state to `save.json` and restore it on load.
//!
//! # Design
//! - `SaveData` is a flat, serde-serializable mirror of all important game resources.
//! - `NpcFriendship` uses `HashMap<Entity, f32>` internally; entities aren't stable
//!   across sessions, so we bridge via the `NpcId(usize)` component (0=Alex, 1=Sam, 2=Mia).
//! - Saving is triggered by a `SaveRequest` event sent by `on_new_day`.
//! - Loading populates a `PendingLoad` resource; `apply_save_data` applies it once
//!   after the world entities have been spawned (OnEnter Playing state).

use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    components::{Furnishings, LocalPlayer, NpcId, PetKind, Vehicle},
    menu::GameStartKind,
    resources::{
        ActionPrompt, BankInput, Conditions, CrisisKind, CrisisState, DailyGoal, FestivalState,
        GameState, GameTime, Hobbies, HousingTier, Inventory, Investment, LifeRating,
        LightningTimer, Milestones, NarrativeState, NearbyInteractable, Notification,
        NpcFriendship, Pet, PlayerMovement, PlayerStats, QuestBoard, Reputation, Season,
        SeasonKind, Skills, SocialEvents, Transport, TransportKind, VehicleState, WeatherKind,
        WorkStreak,
    },
};

#[cfg(not(target_arch = "wasm32"))]
const SAVE_PATH: &str = "save.json";
#[cfg(target_arch = "wasm32")]
const SAVE_STORAGE_KEY: &str = "new_game.save.json";

#[cfg(target_arch = "wasm32")]
fn browser_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn read_save_text() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        browser_storage()?.get_item(SAVE_STORAGE_KEY).ok().flatten()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(SAVE_PATH).ok()
    }
}

fn write_save_text(contents: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) = browser_storage() else {
            return Err("browser localStorage is unavailable".to_string());
        };
        storage
            .set_item(SAVE_STORAGE_KEY, contents)
            .map_err(|e| format!("{e:?}"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::write(SAVE_PATH, contents).map_err(|e| e.to_string())
    }
}

// ── Event ─────────────────────────────────────────────────────────────────────

/// Send this event to trigger an async-style save at the end of the frame.
#[derive(Event, Default)]
pub struct SaveRequest;

// ── Pending load ──────────────────────────────────────────────────────────────

/// Staging area for save data loaded from disk. Applied by `apply_save_data`
/// in the first frame after entering the Playing state.
#[derive(Resource, Default)]
pub struct PendingLoad(pub Option<SaveData>);

// ── SaveDataLegacy (v1 flat format - for migration only) ─────────────────────

#[derive(Serialize, Deserialize, Default, Clone)]
struct SaveDataLegacy {
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
    pub unpaid_rent_days: u32,
    pub meditation_buff: f32,
    pub day: u32,
    pub skill_cooking: f32,
    pub skill_career: f32,
    pub skill_fitness: f32,
    pub skill_social: f32,
    pub streak_days: u32,
    #[serde(default)]
    pub promotion_notified: u8,
    pub housing: u8,
    pub inv_coffee: u32,
    pub inv_vitamins: u32,
    pub inv_books: u32,
    pub inv_coffee_age: u32,
    #[serde(default)]
    pub inv_ingredient: u32,
    #[serde(default)]
    pub inv_gift_box: u32,
    #[serde(default)]
    pub inv_smoothie: u32,
    pub ms_saved_100: bool,
    pub ms_exec: bool,
    pub ms_best_friend: bool,
    pub ms_streak_7: bool,
    pub ms_rating_a: bool,
    pub ms_debt_free: bool,
    pub ms_penthouse: bool,
    pub ms_investor: bool,
    pub ms_hobbyist: bool,
    pub ms_famous: bool,
    pub ms_scholar: bool,
    pub ms_pet_owner: bool,
    pub ms_party_animal: bool,
    pub ms_commuter: bool,
    pub ms_all_seasons: bool,
    #[serde(default)]
    pub ms_quest_master: bool,
    #[serde(default)]
    pub ms_master_chef: bool,
    #[serde(default)]
    pub ms_gift_giver: bool,
    #[serde(default)]
    pub ms_popular: bool,
    #[serde(default)]
    pub ms_crisis_survivor: bool,
    pub rating_score: f32,
    pub rating_days: u32,
    pub hobby_painting: f32,
    pub hobby_gaming: f32,
    pub hobby_music: f32,
    pub burnout: bool,
    pub burnout_days: u32,
    pub malnourished: bool,
    pub malnourish_days: u32,
    pub hospitalized: bool,
    pub hospital_timer: f32,
    #[serde(default)]
    pub mental_fatigue: bool,
    #[serde(default)]
    pub high_stress_days: u32,
    #[serde(default)]
    pub low_stress_days: u32,
    pub invest_amount: f32,
    pub invest_risk: u8,
    pub invest_rate: f32,
    pub invest_total_return: f32,
    pub rep_score: f32,
    pub transport: u8,
    #[serde(default)]
    pub transport_work_uses: u32,
    #[serde(default)]
    pub transport_maintenance_due: bool,
    pub has_pet: bool,
    pub pet_hunger: f32,
    pub pet_name: String,
    #[serde(default)]
    pub pet_kind: u8,
    pub parties_thrown: u32,
    pub npc_friendship: [f32; 6],
    pub days_survived: u32,
    #[serde(default)]
    pub quests_completed_total: u32,
    #[serde(default)]
    pub total_gifts: u32,
    #[serde(default)]
    pub total_crafted: u32,
    #[serde(default)]
    pub total_quests: u32,
    #[serde(default)]
    pub crisis_kind: u8,
    #[serde(default)]
    pub crisis_days_left: u32,
    #[serde(default)]
    pub crises_survived: u32,
    #[serde(default)]
    pub crisis_last_day: u32,
    #[serde(default)]
    pub has_insurance: bool,
    #[serde(default)]
    pub insurance_days: u32,
    #[serde(default)]
    pub festival_tokens: u32,
    #[serde(default)]
    pub festival_spring: bool,
    #[serde(default)]
    pub festival_summer: bool,
    #[serde(default)]
    pub festival_autumn: bool,
    #[serde(default)]
    pub festival_winter: bool,
    #[serde(default)]
    pub festivals_total: u32,
    #[serde(default)]
    pub ms_festival_goer: bool,
    #[serde(default)]
    pub furnishing_desk: bool,
    #[serde(default)]
    pub furnishing_bed: bool,
    #[serde(default)]
    pub furnishing_kitchen: bool,
}

impl SaveDataLegacy {
    fn into_v2(self) -> SaveData {
        let player = PlayerSave {
            player_id: 0,
            energy: self.energy,
            hunger: self.hunger,
            happiness: self.happiness,
            health: self.health,
            stress: self.stress,
            sleep_debt: self.sleep_debt,
            money: self.money,
            savings: self.savings,
            loan: self.loan,
            meals: self.meals,
            unpaid_rent_days: self.unpaid_rent_days,
            meditation_buff: self.meditation_buff,
            skill_cooking: self.skill_cooking,
            skill_career: self.skill_career,
            skill_fitness: self.skill_fitness,
            skill_social: self.skill_social,
            streak_days: self.streak_days,
            promotion_notified: self.promotion_notified,
            housing: self.housing,
            inv_coffee: self.inv_coffee,
            inv_vitamins: self.inv_vitamins,
            inv_books: self.inv_books,
            inv_coffee_age: self.inv_coffee_age,
            inv_ingredient: self.inv_ingredient,
            inv_gift_box: self.inv_gift_box,
            inv_smoothie: self.inv_smoothie,
            ms_saved_100: self.ms_saved_100,
            ms_exec: self.ms_exec,
            ms_best_friend: self.ms_best_friend,
            ms_streak_7: self.ms_streak_7,
            ms_rating_a: self.ms_rating_a,
            ms_debt_free: self.ms_debt_free,
            ms_penthouse: self.ms_penthouse,
            ms_investor: self.ms_investor,
            ms_hobbyist: self.ms_hobbyist,
            ms_famous: self.ms_famous,
            ms_scholar: self.ms_scholar,
            ms_pet_owner: self.ms_pet_owner,
            ms_party_animal: self.ms_party_animal,
            ms_commuter: self.ms_commuter,
            ms_all_seasons: self.ms_all_seasons,
            ms_quest_master: self.ms_quest_master,
            ms_master_chef: self.ms_master_chef,
            ms_gift_giver: self.ms_gift_giver,
            ms_popular: self.ms_popular,
            ms_crisis_survivor: self.ms_crisis_survivor,
            ms_festival_goer: self.ms_festival_goer,
            rating_score: self.rating_score,
            rating_days: self.rating_days,
            hobby_painting: self.hobby_painting,
            hobby_gaming: self.hobby_gaming,
            hobby_music: self.hobby_music,
            burnout: self.burnout,
            burnout_days: self.burnout_days,
            malnourished: self.malnourished,
            malnourish_days: self.malnourish_days,
            hospitalized: self.hospitalized,
            hospital_timer: self.hospital_timer,
            mental_fatigue: self.mental_fatigue,
            high_stress_days: self.high_stress_days,
            low_stress_days: self.low_stress_days,
            invest_amount: self.invest_amount,
            invest_risk: self.invest_risk,
            invest_rate: self.invest_rate,
            invest_total_return: self.invest_total_return,
            rep_score: self.rep_score,
            transport: self.transport,
            transport_work_uses: self.transport_work_uses,
            transport_maintenance_due: self.transport_maintenance_due,
            has_pet: self.has_pet,
            pet_hunger: self.pet_hunger,
            pet_name: self.pet_name,
            pet_kind: self.pet_kind,
            parties_thrown: self.parties_thrown,
            npc_friendship: self.npc_friendship,
            days_survived: self.days_survived,
        };
        let world = WorldSave {
            day: self.day,
            quests_completed_total: self.quests_completed_total,
            total_gifts: self.total_gifts,
            total_crafted: self.total_crafted,
            total_quests: self.total_quests,
            crisis_kind: self.crisis_kind,
            crisis_days_left: self.crisis_days_left,
            crises_survived: self.crises_survived,
            crisis_last_day: self.crisis_last_day,
            has_insurance: self.has_insurance,
            insurance_days: self.insurance_days,
            festival_tokens: self.festival_tokens,
            festival_spring: self.festival_spring,
            festival_summer: self.festival_summer,
            festival_autumn: self.festival_autumn,
            festival_winter: self.festival_winter,
            festivals_total: self.festivals_total,
            furnishing_desk: self.furnishing_desk,
            furnishing_bed: self.furnishing_bed,
            furnishing_kitchen: self.furnishing_kitchen,
        };
        SaveData {
            version: 2,
            world,
            players: vec![player],
        }
    }
}

// ── PlayerSave ────────────────────────────────────────────────────────────────

/// Per-player persistent state. Index 0 is the local player.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PlayerSave {
    /// Stable player identifier. 0 = local / single-player.
    #[serde(default)]
    pub player_id: u32,

    // ── PlayerStats ───────────────────────────────────────────────────────────
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
    pub unpaid_rent_days: u32,
    pub meditation_buff: f32,

    // ── Skills ────────────────────────────────────────────────────────────────
    pub skill_cooking: f32,
    pub skill_career: f32,
    pub skill_fitness: f32,
    pub skill_social: f32,

    // ── WorkStreak ────────────────────────────────────────────────────────────
    pub streak_days: u32,
    #[serde(default)]
    pub promotion_notified: u8,

    // ── HousingTier ───────────────────────────────────────────────────────────
    pub housing: u8,

    // ── Inventory ─────────────────────────────────────────────────────────────
    pub inv_coffee: u32,
    pub inv_vitamins: u32,
    pub inv_books: u32,
    pub inv_coffee_age: u32,
    #[serde(default)]
    pub inv_ingredient: u32,
    #[serde(default)]
    pub inv_gift_box: u32,
    #[serde(default)]
    pub inv_smoothie: u32,

    // ── Milestones ────────────────────────────────────────────────────────────
    pub ms_saved_100: bool,
    pub ms_exec: bool,
    pub ms_best_friend: bool,
    pub ms_streak_7: bool,
    pub ms_rating_a: bool,
    pub ms_debt_free: bool,
    pub ms_penthouse: bool,
    pub ms_investor: bool,
    pub ms_hobbyist: bool,
    pub ms_famous: bool,
    pub ms_scholar: bool,
    pub ms_pet_owner: bool,
    pub ms_party_animal: bool,
    pub ms_commuter: bool,
    pub ms_all_seasons: bool,
    #[serde(default)]
    pub ms_quest_master: bool,
    #[serde(default)]
    pub ms_master_chef: bool,
    #[serde(default)]
    pub ms_gift_giver: bool,
    #[serde(default)]
    pub ms_popular: bool,
    #[serde(default)]
    pub ms_crisis_survivor: bool,
    #[serde(default)]
    pub ms_festival_goer: bool,

    // ── LifeRating ────────────────────────────────────────────────────────────
    pub rating_score: f32,
    pub rating_days: u32,

    // ── Hobbies ───────────────────────────────────────────────────────────────
    pub hobby_painting: f32,
    pub hobby_gaming: f32,
    pub hobby_music: f32,

    // ── Conditions ────────────────────────────────────────────────────────────
    pub burnout: bool,
    pub burnout_days: u32,
    pub malnourished: bool,
    pub malnourish_days: u32,
    pub hospitalized: bool,
    pub hospital_timer: f32,
    #[serde(default)]
    pub mental_fatigue: bool,
    #[serde(default)]
    pub high_stress_days: u32,
    #[serde(default)]
    pub low_stress_days: u32,

    // ── Investment ────────────────────────────────────────────────────────────
    pub invest_amount: f32,
    pub invest_risk: u8,
    pub invest_rate: f32,
    pub invest_total_return: f32,

    // ── Reputation ────────────────────────────────────────────────────────────
    pub rep_score: f32,

    // ── Transport ─────────────────────────────────────────────────────────────
    pub transport: u8,
    #[serde(default)]
    pub transport_work_uses: u32,
    #[serde(default)]
    pub transport_maintenance_due: bool,

    // ── Pet ───────────────────────────────────────────────────────────────────
    pub has_pet: bool,
    pub pet_hunger: f32,
    pub pet_name: String,
    #[serde(default)]
    pub pet_kind: u8,

    // ── SocialEvents ──────────────────────────────────────────────────────────
    pub parties_thrown: u32,

    // ── NPC Friendship ────────────────────────────────────────────────────────
    pub npc_friendship: [f32; 6],

    // ── GameState ─────────────────────────────────────────────────────────────
    pub days_survived: u32,
}

// ── WorldSave ─────────────────────────────────────────────────────────────────

/// Shared world state, independent of which player is active.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct WorldSave {
    // ── GameTime ──────────────────────────────────────────────────────────────
    pub day: u32,

    // ── Quests ────────────────────────────────────────────────────────────────
    #[serde(default)]
    pub quests_completed_total: u32,
    #[serde(default)]
    pub total_gifts: u32,
    #[serde(default)]
    pub total_crafted: u32,
    #[serde(default)]
    pub total_quests: u32,

    // ── Crisis ────────────────────────────────────────────────────────────────
    #[serde(default)]
    pub crisis_kind: u8,
    #[serde(default)]
    pub crisis_days_left: u32,
    #[serde(default)]
    pub crises_survived: u32,
    #[serde(default)]
    pub crisis_last_day: u32,
    #[serde(default)]
    pub has_insurance: bool,
    #[serde(default)]
    pub insurance_days: u32,

    // ── Festival ──────────────────────────────────────────────────────────────
    #[serde(default)]
    pub festival_tokens: u32,
    #[serde(default)]
    pub festival_spring: bool,
    #[serde(default)]
    pub festival_summer: bool,
    #[serde(default)]
    pub festival_autumn: bool,
    #[serde(default)]
    pub festival_winter: bool,
    #[serde(default)]
    pub festivals_total: u32,

    // ── Furnishings ───────────────────────────────────────────────────────────
    #[serde(default)]
    pub furnishing_desk: bool,
    #[serde(default)]
    pub furnishing_bed: bool,
    #[serde(default)]
    pub furnishing_kitchen: bool,
}

// ── SaveData (v2) ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct SaveData {
    /// Format version. 1 = legacy flat format (handled by SaveDataLegacy),
    /// 2 = current per-player format.
    pub version: u32,
    /// Shared world state.
    pub world: WorldSave,
    /// Per-player state. `players[0]` is always the local/single player.
    pub players: Vec<PlayerSave>,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: 2,
            world: WorldSave::default(),
            players: vec![PlayerSave::default()],
        }
    }
}

impl SaveData {
    /// Convenience accessor for the local player save slot.
    pub fn local_player(&self) -> &PlayerSave {
        self.players
            .first()
            .expect("SaveData must have at least one player")
    }

    /// Mutable convenience accessor for the local player save slot.
    #[allow(dead_code)]
    pub fn local_player_mut(&mut self) -> &mut PlayerSave {
        self.players
            .first_mut()
            .expect("SaveData must have at least one player")
    }
}

// ── SystemParam groups (Bevy 16-param limit) ──────────────────────────────────

#[derive(SystemParam)]
pub struct SaveParamsA<'w, 's> {
    pub gt: Res<'w, GameTime>,
    pub ms: Res<'w, Milestones>,
    pub rating: Res<'w, LifeRating>,
    pub player_q: Query<
        'w,
        's,
        (
            &'static PlayerStats,
            &'static Skills,
            &'static WorkStreak,
            &'static HousingTier,
            &'static Inventory,
            &'static Furnishings,
        ),
        With<LocalPlayer>,
    >,
}

#[derive(SystemParam)]
pub struct SaveParamsB<'w> {
    pub hobbies: Res<'w, Hobbies>,
    pub conds: Res<'w, Conditions>,
    pub invest: Res<'w, Investment>,
    pub rep: Res<'w, Reputation>,
    pub transport: Res<'w, Transport>,
    pub pet: Res<'w, Pet>,
    pub social: Res<'w, SocialEvents>,
    pub gs: Res<'w, GameState>,
    pub quest_board: Res<'w, QuestBoard>,
    pub crisis: Res<'w, CrisisState>,
    pub festival: Res<'w, FestivalState>,
}

#[derive(SystemParam)]
pub struct ApplyParamsA<'w, 's> {
    pub gt: ResMut<'w, GameTime>,
    pub ms: ResMut<'w, Milestones>,
    pub rating: ResMut<'w, LifeRating>,
    pub player_q: Query<
        'w,
        's,
        (
            &'static mut PlayerStats,
            &'static mut Skills,
            &'static mut WorkStreak,
            &'static mut HousingTier,
            &'static mut Inventory,
            &'static mut Furnishings,
        ),
        With<LocalPlayer>,
    >,
}

#[derive(SystemParam)]
pub struct ApplyParamsB<'w> {
    pub hobbies: ResMut<'w, Hobbies>,
    pub conds: ResMut<'w, Conditions>,
    pub invest: ResMut<'w, Investment>,
    pub rep: ResMut<'w, Reputation>,
    pub transport: ResMut<'w, Transport>,
    pub pet: ResMut<'w, Pet>,
    pub social: ResMut<'w, SocialEvents>,
    pub gs: ResMut<'w, GameState>,
    pub season: ResMut<'w, Season>,
    pub quest_board: ResMut<'w, QuestBoard>,
    pub crisis: ResMut<'w, CrisisState>,
    pub festival: ResMut<'w, FestivalState>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns `true` if a save file exists in the active platform store.
pub fn save_exists() -> bool {
    read_save_text().is_some()
}

/// Load save data from storage, migrating from legacy format if needed.
/// Returns `None` if no save exists or data is completely unparseable.
pub fn load_save_data() -> Option<SaveData> {
    let contents = read_save_text()?;
    // Try current v2 format first.
    if let Ok(data) = serde_json::from_str::<SaveData>(&contents) {
        return Some(data);
    }
    // Fall back to legacy v1 flat format and migrate.
    serde_json::from_str::<SaveDataLegacy>(&contents)
        .ok()
        .map(SaveDataLegacy::into_v2)
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Listens for `SaveRequest` events and writes the current game state to disk.
pub fn handle_save(
    mut events: EventReader<SaveRequest>,
    a: SaveParamsA,
    b: SaveParamsB,
    friendship: Res<NpcFriendship>,
    npc_q: Query<(Entity, &NpcId)>,
    mut notif: ResMut<Notification>,
) {
    if events.is_empty() {
        return;
    }
    events.clear();

    let Some((stats, skills, streak, housing, inv, furnishings)) = a.player_q.iter().next() else {
        return;
    };

    let mut npc_friendship = [0f32; 6];
    for (entity, npc_id) in &npc_q {
        if npc_id.0 < 6 {
            npc_friendship[npc_id.0] = friendship.levels.get(&entity).copied().unwrap_or(0.);
        }
    }

    let player = PlayerSave {
        player_id: 0,
        energy: stats.energy,
        hunger: stats.hunger,
        happiness: stats.happiness,
        health: stats.health,
        stress: stats.stress,
        sleep_debt: stats.sleep_debt,
        money: stats.money,
        savings: stats.savings,
        loan: stats.loan,
        meals: stats.meals,
        unpaid_rent_days: stats.unpaid_rent_days,
        meditation_buff: stats.meditation_buff,
        skill_cooking: skills.cooking,
        skill_career: skills.career,
        skill_fitness: skills.fitness,
        skill_social: skills.social,
        streak_days: streak.days,
        promotion_notified: streak.promotion_notified,
        housing: u8::from(housing),
        inv_coffee: inv.coffee,
        inv_vitamins: inv.vitamins,
        inv_books: inv.books,
        inv_coffee_age: inv.coffee_age,
        inv_ingredient: inv.ingredient,
        inv_gift_box: inv.gift_box,
        inv_smoothie: inv.smoothie,
        ms_saved_100: a.ms.saved_100,
        ms_exec: a.ms.exec,
        ms_best_friend: a.ms.best_friend,
        ms_streak_7: a.ms.streak_7,
        ms_rating_a: a.ms.rating_a,
        ms_debt_free: a.ms.debt_free,
        ms_penthouse: a.ms.penthouse,
        ms_investor: a.ms.investor,
        ms_hobbyist: a.ms.hobbyist,
        ms_famous: a.ms.famous,
        ms_scholar: a.ms.scholar,
        ms_pet_owner: a.ms.pet_owner,
        ms_party_animal: a.ms.party_animal,
        ms_commuter: a.ms.commuter,
        ms_all_seasons: a.ms.all_seasons,
        ms_quest_master: a.ms.quest_master,
        ms_master_chef: a.ms.master_chef,
        ms_gift_giver: a.ms.gift_giver,
        ms_popular: a.ms.popular,
        ms_crisis_survivor: a.ms.crisis_survivor,
        ms_festival_goer: a.ms.festival_goer,
        rating_score: a.rating.score,
        rating_days: a.rating.days,
        hobby_painting: b.hobbies.painting,
        hobby_gaming: b.hobbies.gaming,
        hobby_music: b.hobbies.music,
        burnout: b.conds.burnout,
        burnout_days: b.conds.burnout_days,
        malnourished: b.conds.malnourished,
        malnourish_days: b.conds.malnourish_days,
        hospitalized: b.conds.hospitalized,
        hospital_timer: b.conds.hospital_timer,
        mental_fatigue: b.conds.mental_fatigue,
        high_stress_days: b.conds.high_stress_days,
        low_stress_days: b.conds.low_stress_days,
        invest_amount: b.invest.amount,
        invest_risk: b.invest.risk,
        invest_rate: b.invest.daily_return_rate,
        invest_total_return: b.invest.total_return,
        rep_score: b.rep.score,
        transport: u8::from(&b.transport.kind),
        transport_work_uses: b.transport.work_uses,
        transport_maintenance_due: b.transport.maintenance_due,
        has_pet: b.pet.has_pet,
        pet_hunger: b.pet.hunger,
        pet_name: b.pet.name.clone(),
        pet_kind: u8::from(&b.pet.kind),
        parties_thrown: b.social.parties_thrown,
        npc_friendship,
        days_survived: b.gs.days_survived,
    };
    let world = WorldSave {
        day: a.gt.day,
        quests_completed_total: b.quest_board.completed_total,
        total_gifts: b.gs.total_gifts,
        total_crafted: b.gs.total_crafted,
        total_quests: b.gs.total_quests,
        crisis_kind: b.crisis.active.as_ref().map(u8::from).unwrap_or(0),
        crisis_days_left: b.crisis.days_left,
        crises_survived: b.crisis.crises_survived,
        crisis_last_day: b.crisis.last_crisis_day,
        has_insurance: b.crisis.has_insurance,
        insurance_days: b.crisis.insurance_days,
        festival_tokens: b.festival.tokens,
        festival_spring: b.festival.spring_attended,
        festival_summer: b.festival.summer_attended,
        festival_autumn: b.festival.autumn_attended,
        festival_winter: b.festival.winter_attended,
        festivals_total: b.festival.festivals_total,
        furnishing_desk: furnishings.desk,
        furnishing_bed: furnishings.bed,
        furnishing_kitchen: furnishings.kitchen,
    };
    let data = SaveData {
        version: 2,
        world,
        players: vec![player],
    };

    match serde_json::to_string_pretty(&data) {
        Ok(json) => {
            if let Err(e) = write_save_text(&json) {
                eprintln!("[save] Write failed: {e}");
                notif.push("Save failed - could not write progress", 4.0);
            }
        }
        Err(e) => {
            eprintln!("[save] Serialize failed: {e}");
            notif.push("Save failed - serialization error", 4.0);
        }
    }
}

/// Applies `PendingLoad` data. Only runs when `GameStartKind::Continue` was set;
/// otherwise clears any leftover pending data and returns.
pub fn apply_save_data(
    start_kind: Res<GameStartKind>,
    mut pending: ResMut<PendingLoad>,
    mut a: ApplyParamsA,
    mut b: ApplyParamsB,
    mut friendship: ResMut<NpcFriendship>,
    npc_q: Query<(Entity, &NpcId)>,
    mut vehicle_q: Query<&mut Visibility, With<Vehicle>>,
) {
    if *start_kind != GameStartKind::Continue {
        pending.0 = None;
        return;
    }
    let Some(data) = pending.0.take() else { return };

    let p = data.local_player();
    let w = &data.world;

    let Some((mut stats, mut skills, mut streak, mut housing, mut inv, mut furnishings)) =
        a.player_q.iter_mut().next()
    else {
        return;
    };

    // ── PlayerStats ───────────────────────────────────────────────────────────
    stats.energy = p.energy;
    stats.hunger = p.hunger;
    stats.happiness = p.happiness;
    stats.health = p.health;
    stats.stress = p.stress;
    stats.sleep_debt = p.sleep_debt;
    stats.money = p.money;
    stats.savings = p.savings;
    stats.loan = p.loan;
    stats.meals = p.meals;
    stats.unpaid_rent_days = p.unpaid_rent_days;
    stats.meditation_buff = p.meditation_buff;

    // ── GameTime ──────────────────────────────────────────────────────────────
    a.gt.day = w.day;
    a.gt.prev_day = w.day; // prevent false new-day trigger
    a.gt.hours = 8.; // start each loaded session in the morning

    // ── Skills ────────────────────────────────────────────────────────────────
    skills.cooking = p.skill_cooking;
    skills.career = p.skill_career;
    skills.fitness = p.skill_fitness;
    skills.social = p.skill_social;

    // ── WorkStreak ────────────────────────────────────────────────────────────
    streak.days = p.streak_days;
    streak.promotion_notified = p.promotion_notified;

    // ── HousingTier ───────────────────────────────────────────────────────────
    *housing = HousingTier::from(p.housing);

    // ── Furnishings ───────────────────────────────────────────────────────────
    furnishings.desk = w.furnishing_desk;
    furnishings.bed = w.furnishing_bed;
    furnishings.kitchen = w.furnishing_kitchen;

    // ── Inventory ─────────────────────────────────────────────────────────────
    inv.coffee = p.inv_coffee;
    inv.vitamins = p.inv_vitamins;
    inv.books = p.inv_books;
    inv.coffee_age = p.inv_coffee_age;
    inv.ingredient = p.inv_ingredient;
    inv.gift_box = p.inv_gift_box;
    inv.smoothie = p.inv_smoothie;

    // ── Milestones ────────────────────────────────────────────────────────────
    a.ms.saved_100 = p.ms_saved_100;
    a.ms.exec = p.ms_exec;
    a.ms.best_friend = p.ms_best_friend;
    a.ms.streak_7 = p.ms_streak_7;
    a.ms.rating_a = p.ms_rating_a;
    a.ms.debt_free = p.ms_debt_free;
    a.ms.penthouse = p.ms_penthouse;
    a.ms.investor = p.ms_investor;
    a.ms.hobbyist = p.ms_hobbyist;
    a.ms.famous = p.ms_famous;
    a.ms.scholar = p.ms_scholar;
    a.ms.pet_owner = p.ms_pet_owner;
    a.ms.party_animal = p.ms_party_animal;
    a.ms.commuter = p.ms_commuter;
    a.ms.all_seasons = p.ms_all_seasons;
    a.ms.quest_master = p.ms_quest_master;
    a.ms.master_chef = p.ms_master_chef;
    a.ms.gift_giver = p.ms_gift_giver;
    a.ms.popular = p.ms_popular;
    a.ms.crisis_survivor = p.ms_crisis_survivor;
    a.ms.festival_goer = p.ms_festival_goer;

    // ── LifeRating ────────────────────────────────────────────────────────────
    a.rating.score = p.rating_score;
    a.rating.days = p.rating_days;

    // ── Hobbies ───────────────────────────────────────────────────────────────
    b.hobbies.painting = p.hobby_painting;
    b.hobbies.gaming = p.hobby_gaming;
    b.hobbies.music = p.hobby_music;

    // ── Conditions ────────────────────────────────────────────────────────────
    b.conds.burnout = p.burnout;
    b.conds.burnout_days = p.burnout_days;
    b.conds.malnourished = p.malnourished;
    b.conds.malnourish_days = p.malnourish_days;
    b.conds.hospitalized = p.hospitalized;
    b.conds.hospital_timer = p.hospital_timer;
    b.conds.mental_fatigue = p.mental_fatigue;
    b.conds.high_stress_days = p.high_stress_days;
    b.conds.low_stress_days = p.low_stress_days;

    // ── Investment ────────────────────────────────────────────────────────────
    b.invest.amount = p.invest_amount;
    b.invest.risk = p.invest_risk;
    b.invest.daily_return_rate = p.invest_rate;
    b.invest.total_return = p.invest_total_return;

    // ── Reputation ────────────────────────────────────────────────────────────
    b.rep.score = p.rep_score;

    // ── Transport ─────────────────────────────────────────────────────────────
    b.transport.kind = TransportKind::from(p.transport);
    b.transport.work_uses = p.transport_work_uses;
    b.transport.maintenance_due = p.transport_maintenance_due;
    if b.transport.kind == TransportKind::Car {
        for mut vis in &mut vehicle_q {
            *vis = Visibility::Visible;
        }
    }

    // ── Pet ───────────────────────────────────────────────────────────────────
    b.pet.has_pet = p.has_pet;
    b.pet.hunger = p.pet_hunger;
    b.pet.name = p.pet_name.clone();
    b.pet.fed_today = false;
    b.pet.kind = PetKind::from(p.pet_kind);

    // ── SocialEvents ──────────────────────────────────────────────────────────
    b.social.parties_thrown = p.parties_thrown;

    // ── Season (derived from day) ─────────────────────────────────────────────
    b.season.current = SeasonKind::from_day(w.day);

    // ── GameState ─────────────────────────────────────────────────────────────
    b.gs.days_survived = p.days_survived;

    // ── Quests ────────────────────────────────────────────────────────────────
    b.quest_board.completed_total = w.quests_completed_total;
    b.gs.total_gifts = w.total_gifts;
    b.gs.total_crafted = w.total_crafted;
    b.gs.total_quests = w.total_quests;

    // ── Crisis ────────────────────────────────────────────────────────────────
    b.crisis.active = CrisisKind::from_u8(w.crisis_kind);
    b.crisis.days_left = w.crisis_days_left;
    b.crisis.crises_survived = w.crises_survived;
    b.crisis.last_crisis_day = w.crisis_last_day;
    b.crisis.has_insurance = w.has_insurance;
    b.crisis.insurance_days = w.insurance_days;

    // ── Festival ──────────────────────────────────────────────────────────────
    b.festival.tokens = w.festival_tokens;
    b.festival.spring_attended = w.festival_spring;
    b.festival.summer_attended = w.festival_summer;
    b.festival.autumn_attended = w.festival_autumn;
    b.festival.winter_attended = w.festival_winter;
    b.festival.festivals_total = w.festivals_total;

    // ── NPC Friendship ────────────────────────────────────────────────────────
    friendship.levels.clear();
    for (entity, npc_id) in &npc_q {
        if npc_id.0 < 6 {
            friendship.levels.insert(entity, p.npc_friendship[npc_id.0]);
        }
    }
}

/// Resets every game resource to its default value. Runs at the start of each
/// new Playing session (NewGame or Continue) so state is always clean.
/// Skipped on Resume (Paused -> Playing) via the GameStartKind check.
#[allow(clippy::too_many_arguments)]
pub fn reset_game(
    start_kind: Res<GameStartKind>,
    mut a: ApplyParamsA,
    mut b: ApplyParamsB,
    mut friendship: ResMut<NpcFriendship>,
    mut notif: ResMut<Notification>,
    mut narrative: ResMut<NarrativeState>,
    mut weather: ResMut<WeatherKind>,
    mut nearby: ResMut<NearbyInteractable>,
    mut player_state_q: Query<
        (
            &mut PlayerMovement,
            &mut VehicleState,
            &mut BankInput,
            &mut ActionPrompt,
        ),
        With<LocalPlayer>,
    >,
    mut goal: ResMut<DailyGoal>,
    mut lightning: ResMut<LightningTimer>,
    npc_q: Query<(Entity, &NpcId)>,
) {
    if *start_kind == GameStartKind::Resume {
        return;
    }

    if let Some((mut stats, mut skills, mut streak, mut housing, mut inv, mut furnishings)) =
        a.player_q.iter_mut().next()
    {
        *stats = PlayerStats::default();
        *skills = Skills::default();
        *streak = WorkStreak::default();
        *housing = HousingTier::default();
        *inv = Inventory::default();
        *furnishings = Furnishings::default();
    }
    *a.gt = GameTime::default();
    *a.ms = Milestones::default();
    *a.rating = LifeRating::default();
    *b.hobbies = Hobbies::default();
    *b.conds = Conditions::default();
    *b.invest = Investment::default();
    *b.rep = Reputation::default();
    *b.transport = Transport::default();
    *b.pet = Pet::default();
    *b.social = SocialEvents::default();
    *b.gs = GameState::default();
    *b.crisis = CrisisState::default();
    *b.festival = FestivalState::default();
    b.season.current = SeasonKind::default();
    *notif = Notification::default();
    *narrative = NarrativeState::default();
    *weather = WeatherKind::default();
    *nearby = NearbyInteractable::default();
    if let Some((mut movement, mut vehicle_state, mut bank_input, mut action_prompt)) =
        player_state_q.iter_mut().next()
    {
        *movement = PlayerMovement::default();
        *vehicle_state = VehicleState::default();
        *bank_input = BankInput::default();
        *action_prompt = ActionPrompt::default();
    }
    *goal = DailyGoal::default();
    *lightning = LightningTimer::default();

    friendship.levels.clear();
    friendship.chatted_today.clear();
    for (entity, npc_id) in &npc_q {
        if npc_id.0 < 6 {
            friendship.levels.insert(entity, 0.);
        }
    }
}

/// Activates the tutorial on a new game. Called in the `OnEnter(Playing)` chain
/// before `reset_start_kind` resets `GameStartKind` back to `Resume`.
pub fn start_tutorial_if_new_game(
    start_kind: Res<GameStartKind>,
    mut tutorial: ResMut<crate::resources::TutorialState>,
) {
    if *start_kind == GameStartKind::NewGame {
        tutorial.step = 1;
    } else {
        tutorial.step = 0;
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::{PlayerSave, SaveData, SaveDataLegacy, WorldSave};

    // Adding a new SaveData field checklist:
    // 1. Add field + #[serde(default)] to PlayerSave or WorldSave
    // 2. Serialize in handle_save (player.field = ... or world.field = ...)
    // 3. Deserialize in apply_save_data (p.field or w.field)
    // 4. Update sample_save() below (compile error if missed)
    fn sample_player() -> PlayerSave {
        PlayerSave {
            player_id: 0,
            energy: 75.5,
            hunger: 30.0,
            happiness: 60.0,
            health: 90.0,
            stress: 25.0,
            sleep_debt: 4.0,
            money: 150.0,
            savings: 500.0,
            loan: 0.0,
            meals: 2,
            unpaid_rent_days: 0,
            meditation_buff: 0.0,
            skill_cooking: 1.5,
            skill_career: 2.0,
            skill_fitness: 0.5,
            skill_social: 1.0,
            streak_days: 3,
            promotion_notified: 0,
            housing: 1,
            inv_coffee: 2,
            inv_vitamins: 1,
            inv_books: 0,
            inv_coffee_age: 2,
            inv_ingredient: 0,
            inv_gift_box: 0,
            inv_smoothie: 0,
            ms_saved_100: false,
            ms_exec: true,
            ms_best_friend: false,
            ms_streak_7: false,
            ms_rating_a: false,
            ms_debt_free: true,
            ms_penthouse: false,
            ms_investor: false,
            ms_hobbyist: false,
            ms_famous: false,
            ms_scholar: false,
            ms_pet_owner: false,
            ms_party_animal: false,
            ms_commuter: false,
            ms_all_seasons: false,
            ms_quest_master: false,
            ms_master_chef: false,
            ms_gift_giver: false,
            ms_popular: false,
            ms_crisis_survivor: false,
            ms_festival_goer: false,
            rating_score: 72.0,
            rating_days: 5,
            hobby_painting: 0.0,
            hobby_gaming: 1.0,
            hobby_music: 0.0,
            burnout: false,
            burnout_days: 0,
            malnourished: false,
            malnourish_days: 0,
            hospitalized: false,
            hospital_timer: 0.0,
            mental_fatigue: false,
            high_stress_days: 0,
            low_stress_days: 0,
            invest_amount: 200.0,
            invest_risk: 0,
            invest_rate: 0.04,
            invest_total_return: 8.0,
            rep_score: 45.0,
            transport: 1,
            transport_work_uses: 0,
            transport_maintenance_due: false,
            has_pet: false,
            pet_hunger: 0.0,
            pet_name: String::new(),
            pet_kind: 0,
            parties_thrown: 1,
            npc_friendship: [2.0, 3.5, 1.0, 0.0, 0.5, 1.5],
            days_survived: 5,
        }
    }

    fn sample_world() -> WorldSave {
        WorldSave {
            day: 5,
            quests_completed_total: 0,
            total_gifts: 0,
            total_crafted: 0,
            total_quests: 0,
            crisis_kind: 0,
            crisis_days_left: 0,
            crises_survived: 0,
            crisis_last_day: 0,
            has_insurance: false,
            insurance_days: 0,
            festival_tokens: 0,
            festival_spring: false,
            festival_summer: false,
            festival_autumn: false,
            festival_winter: false,
            festivals_total: 0,
            furnishing_desk: false,
            furnishing_bed: false,
            furnishing_kitchen: false,
        }
    }

    fn sample_save() -> SaveData {
        SaveData {
            version: 2,
            world: sample_world(),
            players: vec![sample_player()],
        }
    }

    #[test]
    fn save_data_round_trips_via_json() {
        let original = sample_save();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize");

        let op = original.local_player();
        let rp = restored.local_player();
        assert!((op.energy - rp.energy).abs() < f32::EPSILON);
        assert!((op.money - rp.money).abs() < f32::EPSILON);
        assert_eq!(original.world.day, restored.world.day);
        assert_eq!(op.housing, rp.housing);
        assert_eq!(op.ms_exec, rp.ms_exec);
        assert_eq!(op.ms_debt_free, rp.ms_debt_free);
        assert_eq!(op.transport, rp.transport);
        assert!((op.npc_friendship[1] - rp.npc_friendship[1]).abs() < f32::EPSILON);
    }

    #[test]
    fn save_data_default_is_valid_json() {
        let data = SaveData::default();
        let json = serde_json::to_string(&data).expect("serialize default");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize default");
        assert_eq!(restored.version, 2);
        assert_eq!(restored.players.len(), 1);
    }

    #[test]
    fn penthouse_housing_round_trips() {
        let mut data = sample_save();
        data.local_player_mut().housing = 3; // Penthouse
        let json = serde_json::to_string(&data).expect("serialize");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.local_player().housing, 3);
    }

    #[test]
    fn pet_kind_fish_round_trips() {
        let mut data = sample_save();
        {
            let p = data.local_player_mut();
            p.has_pet = true;
            p.pet_kind = 2; // Fish
            p.pet_name = "Nemo".to_string();
            p.pet_hunger = 15.;
        }
        let json = serde_json::to_string(&data).expect("serialize");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize");
        let rp = restored.local_player();
        assert_eq!(rp.pet_kind, 2);
        assert_eq!(rp.pet_name, "Nemo");
        assert!((rp.pet_hunger - 15.).abs() < f32::EPSILON);
    }

    #[test]
    fn friendship_boundary_values_round_trip() {
        let mut data = sample_save();
        data.local_player_mut().npc_friendship = [0., 5., 0., 5., 2.5, 0.];
        let json = serde_json::to_string(&data).expect("serialize");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize");
        let rp = restored.local_player();
        assert!((rp.npc_friendship[0] - 0.).abs() < f32::EPSILON);
        assert!((rp.npc_friendship[1] - 5.).abs() < f32::EPSILON);
        assert!((rp.npc_friendship[4] - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn corrupted_json_does_not_panic() {
        let result = serde_json::from_str::<SaveData>("not valid json");
        assert!(result.is_err(), "corrupted JSON should fail gracefully");
    }

    #[test]
    fn truncated_json_does_not_panic() {
        let full = serde_json::to_string(&SaveData::default()).expect("serialize");
        let truncated = &full[..full.len() / 2];
        let result = serde_json::from_str::<SaveData>(truncated);
        assert!(result.is_err(), "truncated JSON should fail gracefully");
    }

    #[test]
    fn all_milestone_flags_round_trip() {
        let mut data = SaveData::default();
        {
            let p = data.local_player_mut();
            p.ms_saved_100 = true;
            p.ms_exec = true;
            p.ms_best_friend = true;
            p.ms_streak_7 = true;
            p.ms_rating_a = true;
            p.ms_debt_free = true;
            p.ms_penthouse = true;
            p.ms_investor = true;
            p.ms_hobbyist = true;
            p.ms_famous = true;
            p.ms_scholar = true;
            p.ms_pet_owner = true;
            p.ms_party_animal = true;
            p.ms_commuter = true;
            p.ms_all_seasons = true;
        }
        let json = serde_json::to_string(&data).expect("serialize");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize");
        let rp = restored.local_player();
        assert!(
            rp.ms_saved_100
                && rp.ms_exec
                && rp.ms_penthouse
                && rp.ms_all_seasons
                && rp.ms_party_animal
        );
    }

    #[test]
    fn legacy_v1_save_migrates_to_v2() {
        // Build a minimal legacy flat save and verify it round-trips through migration.
        let legacy = SaveDataLegacy {
            energy: 80.0,
            money: 250.0,
            day: 10,
            ms_exec: true,
            npc_friendship: [1., 2., 3., 4., 5., 6.],
            ..SaveDataLegacy::default()
        };
        let json = serde_json::to_string(&legacy).expect("serialize legacy");
        // Manually call migration path.
        let migrated: SaveData = serde_json::from_str::<SaveDataLegacy>(&json)
            .expect("deserialize legacy")
            .into_v2();
        assert_eq!(migrated.version, 2);
        assert_eq!(migrated.world.day, 10);
        assert!((migrated.local_player().energy - 80.0).abs() < f32::EPSILON);
        assert!(migrated.local_player().ms_exec);
        assert!((migrated.local_player().npc_friendship[4] - 5.).abs() < f32::EPSILON);
    }
}
