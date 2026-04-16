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
    components::NpcId,
    resources::{
        Conditions, GameState, GameTime, Hobbies, HousingTier, Inventory, Investment,
        LifeRating, Milestones, NpcFriendship, Pet, PlayerStats, Reputation, Season, SeasonKind,
        Skills, SocialEvents, Transport, TransportKind, WorkStreak,
    },
};

const SAVE_PATH: &str = "save.json";

// ── Event ─────────────────────────────────────────────────────────────────────

/// Send this event to trigger an async-style save at the end of the frame.
#[derive(Event, Default)]
pub struct SaveRequest;

// ── Pending load ──────────────────────────────────────────────────────────────

/// Staging area for save data loaded from disk. Applied by `apply_save_data`
/// in the first frame after entering the Playing state.
#[derive(Resource, Default)]
pub struct PendingLoad(pub Option<SaveData>);

// ── SaveData ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SaveData {
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

    // ── GameTime ──────────────────────────────────────────────────────────────
    /// Day number (hours reset to 8 on load).
    pub day: u32,

    // ── Skills ────────────────────────────────────────────────────────────────
    pub skill_cooking: f32,
    pub skill_career: f32,
    pub skill_fitness: f32,
    pub skill_social: f32,

    // ── WorkStreak ────────────────────────────────────────────────────────────
    pub streak_days: u32,

    // ── HousingTier ───────────────────────────────────────────────────────────
    /// 0 = Apartment, 1 = Condo, 2 = Penthouse
    pub housing: u8,

    // ── Inventory ─────────────────────────────────────────────────────────────
    pub inv_coffee: u32,
    pub inv_vitamins: u32,
    pub inv_books: u32,

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

    // ── Investment ────────────────────────────────────────────────────────────
    pub invest_amount: f32,
    pub invest_risk: u8,
    pub invest_rate: f32,
    pub invest_total_return: f32,

    // ── Reputation ────────────────────────────────────────────────────────────
    pub rep_score: f32,

    // ── Transport ─────────────────────────────────────────────────────────────
    /// 0 = Walk, 1 = Bike, 2 = Car
    pub transport: u8,

    // ── Pet ───────────────────────────────────────────────────────────────────
    pub has_pet: bool,
    pub pet_hunger: f32,
    pub pet_name: String,

    // ── SocialEvents ──────────────────────────────────────────────────────────
    pub parties_thrown: u32,

    // ── NPC Friendship ────────────────────────────────────────────────────────
    /// Indexed by NpcId: [Alex(0), Sam(1), Mia(2)]
    pub npc_friendship: [f32; 3],

    // ── GameState ─────────────────────────────────────────────────────────────
    pub days_survived: u32,
}

// ── SystemParam groups (Bevy 16-param limit) ──────────────────────────────────

#[derive(SystemParam)]
pub struct SaveParamsA<'w> {
    pub stats: Res<'w, PlayerStats>,
    pub gt: Res<'w, GameTime>,
    pub skills: Res<'w, Skills>,
    pub streak: Res<'w, WorkStreak>,
    pub housing: Res<'w, HousingTier>,
    pub inv: Res<'w, Inventory>,
    pub ms: Res<'w, Milestones>,
    pub rating: Res<'w, LifeRating>,
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
}

#[derive(SystemParam)]
pub struct ApplyParamsA<'w> {
    pub stats: ResMut<'w, PlayerStats>,
    pub gt: ResMut<'w, GameTime>,
    pub skills: ResMut<'w, Skills>,
    pub streak: ResMut<'w, WorkStreak>,
    pub housing: ResMut<'w, HousingTier>,
    pub inv: ResMut<'w, Inventory>,
    pub ms: ResMut<'w, Milestones>,
    pub rating: ResMut<'w, LifeRating>,
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
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns `true` if a save file exists on disk.
pub fn save_exists() -> bool {
    std::path::Path::new(SAVE_PATH).exists()
}

/// Load raw save data from disk. Returns `None` if no file or parse error.
pub fn load_save_data() -> Option<SaveData> {
    let contents = std::fs::read_to_string(SAVE_PATH).ok()?;
    serde_json::from_str(&contents).ok()
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Listens for `SaveRequest` events and writes the current game state to disk.
pub fn handle_save(
    mut events: EventReader<SaveRequest>,
    a: SaveParamsA,
    b: SaveParamsB,
    friendship: Res<NpcFriendship>,
    npc_q: Query<(Entity, &NpcId)>,
) {
    if events.is_empty() {
        return;
    }
    events.clear();

    let mut npc_friendship = [0f32; 3];
    for (entity, npc_id) in &npc_q {
        if npc_id.0 < 3 {
            npc_friendship[npc_id.0] = friendship.levels.get(&entity).copied().unwrap_or(0.);
        }
    }

    let data = SaveData {
        energy: a.stats.energy,
        hunger: a.stats.hunger,
        happiness: a.stats.happiness,
        health: a.stats.health,
        stress: a.stats.stress,
        sleep_debt: a.stats.sleep_debt,
        money: a.stats.money,
        savings: a.stats.savings,
        loan: a.stats.loan,
        meals: a.stats.meals,
        unpaid_rent_days: a.stats.unpaid_rent_days,
        meditation_buff: a.stats.meditation_buff,
        day: a.gt.day,
        skill_cooking: a.skills.cooking,
        skill_career: a.skills.career,
        skill_fitness: a.skills.fitness,
        skill_social: a.skills.social,
        streak_days: a.streak.days,
        housing: match *a.housing {
            HousingTier::Apartment => 0,
            HousingTier::Condo => 1,
            HousingTier::Penthouse => 2,
        },
        inv_coffee: a.inv.coffee,
        inv_vitamins: a.inv.vitamins,
        inv_books: a.inv.books,
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
        rating_score: a.rating.score,
        rating_days: a.rating.days,
        hobby_painting: b.hobbies.painting,
        hobby_gaming: b.hobbies.gaming,
        hobby_music: b.hobbies.music,
        burnout: b.conds.burnout,
        burnout_days: b.conds.burnout_days,
        malnourished: b.conds.malnourished,
        malnourish_days: b.conds.malnourish_days,
        invest_amount: b.invest.amount,
        invest_risk: b.invest.risk,
        invest_rate: b.invest.daily_return_rate,
        invest_total_return: b.invest.total_return,
        rep_score: b.rep.score,
        transport: match b.transport.kind {
            TransportKind::Walk => 0,
            TransportKind::Bike => 1,
            TransportKind::Car => 2,
        },
        has_pet: b.pet.has_pet,
        pet_hunger: b.pet.hunger,
        pet_name: b.pet.name.clone(),
        parties_thrown: b.social.parties_thrown,
        npc_friendship,
        days_survived: b.gs.days_survived,
    };

    match serde_json::to_string_pretty(&data) {
        Ok(json) => {
            if let Err(e) = std::fs::write(SAVE_PATH, &json) {
                eprintln!("[save] Write failed: {e}");
            }
        }
        Err(e) => eprintln!("[save] Serialize failed: {e}"),
    }
}

/// Applies `PendingLoad` data to all game resources. Runs once on entering
/// the Playing state (via `OnEnter`), after all entities have been spawned.
pub fn apply_save_data(
    mut pending: ResMut<PendingLoad>,
    mut a: ApplyParamsA,
    mut b: ApplyParamsB,
    mut friendship: ResMut<NpcFriendship>,
    npc_q: Query<(Entity, &NpcId)>,
) {
    let Some(data) = pending.0.take() else { return };

    // ── PlayerStats ───────────────────────────────────────────────────────────
    a.stats.energy = data.energy;
    a.stats.hunger = data.hunger;
    a.stats.happiness = data.happiness;
    a.stats.health = data.health;
    a.stats.stress = data.stress;
    a.stats.sleep_debt = data.sleep_debt;
    a.stats.money = data.money;
    a.stats.savings = data.savings;
    a.stats.loan = data.loan;
    a.stats.meals = data.meals;
    a.stats.unpaid_rent_days = data.unpaid_rent_days;
    a.stats.meditation_buff = data.meditation_buff;

    // ── GameTime ──────────────────────────────────────────────────────────────
    a.gt.day = data.day;
    a.gt.prev_day = data.day; // prevent false new-day trigger
    a.gt.hours = 8.;          // start each loaded session in the morning

    // ── Skills ────────────────────────────────────────────────────────────────
    a.skills.cooking = data.skill_cooking;
    a.skills.career = data.skill_career;
    a.skills.fitness = data.skill_fitness;
    a.skills.social = data.skill_social;

    // ── WorkStreak ────────────────────────────────────────────────────────────
    a.streak.days = data.streak_days;

    // ── HousingTier ───────────────────────────────────────────────────────────
    *a.housing = match data.housing {
        1 => HousingTier::Condo,
        2 => HousingTier::Penthouse,
        _ => HousingTier::Apartment,
    };

    // ── Inventory ─────────────────────────────────────────────────────────────
    a.inv.coffee = data.inv_coffee;
    a.inv.vitamins = data.inv_vitamins;
    a.inv.books = data.inv_books;

    // ── Milestones ────────────────────────────────────────────────────────────
    a.ms.saved_100 = data.ms_saved_100;
    a.ms.exec = data.ms_exec;
    a.ms.best_friend = data.ms_best_friend;
    a.ms.streak_7 = data.ms_streak_7;
    a.ms.rating_a = data.ms_rating_a;
    a.ms.debt_free = data.ms_debt_free;
    a.ms.penthouse = data.ms_penthouse;
    a.ms.investor = data.ms_investor;
    a.ms.hobbyist = data.ms_hobbyist;
    a.ms.famous = data.ms_famous;
    a.ms.scholar = data.ms_scholar;
    a.ms.pet_owner = data.ms_pet_owner;
    a.ms.party_animal = data.ms_party_animal;
    a.ms.commuter = data.ms_commuter;
    a.ms.all_seasons = data.ms_all_seasons;

    // ── LifeRating ────────────────────────────────────────────────────────────
    a.rating.score = data.rating_score;
    a.rating.days = data.rating_days;

    // ── Hobbies ───────────────────────────────────────────────────────────────
    b.hobbies.painting = data.hobby_painting;
    b.hobbies.gaming = data.hobby_gaming;
    b.hobbies.music = data.hobby_music;

    // ── Conditions ────────────────────────────────────────────────────────────
    b.conds.burnout = data.burnout;
    b.conds.burnout_days = data.burnout_days;
    b.conds.malnourished = data.malnourished;
    b.conds.malnourish_days = data.malnourish_days;

    // ── Investment ────────────────────────────────────────────────────────────
    b.invest.amount = data.invest_amount;
    b.invest.risk = data.invest_risk;
    b.invest.daily_return_rate = data.invest_rate;
    b.invest.total_return = data.invest_total_return;

    // ── Reputation ────────────────────────────────────────────────────────────
    b.rep.score = data.rep_score;

    // ── Transport ─────────────────────────────────────────────────────────────
    b.transport.kind = match data.transport {
        1 => TransportKind::Bike,
        2 => TransportKind::Car,
        _ => TransportKind::Walk,
    };

    // ── Pet ───────────────────────────────────────────────────────────────────
    b.pet.has_pet = data.has_pet;
    b.pet.hunger = data.pet_hunger;
    b.pet.name = data.pet_name;
    b.pet.fed_today = false;

    // ── SocialEvents ──────────────────────────────────────────────────────────
    b.social.parties_thrown = data.parties_thrown;

    // ── Season (derived from day) ─────────────────────────────────────────────
    b.season.current = SeasonKind::from_day(data.day);

    // ── GameState ─────────────────────────────────────────────────────────────
    b.gs.days_survived = data.days_survived;

    // ── NPC Friendship ────────────────────────────────────────────────────────
    friendship.levels.clear();
    for (entity, npc_id) in &npc_q {
        if npc_id.0 < 3 {
            friendship.levels.insert(entity, data.npc_friendship[npc_id.0]);
        }
    }
}
