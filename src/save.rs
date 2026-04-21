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
    components::{LocalPlayer, NpcId, PetKind, Player, Vehicle},
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
    /// 0 = Unhoused, 1 = Apartment, 2 = Condo, 3 = Penthouse
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
    /// 0 = Walk, 1 = Bike, 2 = Car
    pub transport: u8,
    #[serde(default)]
    pub transport_work_uses: u32,
    #[serde(default)]
    pub transport_maintenance_due: bool,

    // ── Pet ───────────────────────────────────────────────────────────────────
    pub has_pet: bool,
    pub pet_hunger: f32,
    pub pet_name: String,
    /// 0 = Dog, 1 = Cat, 2 = Fish
    #[serde(default)]
    pub pet_kind: u8,

    // ── SocialEvents ──────────────────────────────────────────────────────────
    pub parties_thrown: u32,

    // ── NPC Friendship ────────────────────────────────────────────────────────
    /// Indexed by NpcId: [Alex(0), Sam(1), Mia(2), Jordan(3), Taylor(4), Casey(5)]
    pub npc_friendship: [f32; 6],

    // ── GameState ─────────────────────────────────────────────────────────────
    pub days_survived: u32,

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
    #[serde(default)]
    pub ms_festival_goer: bool,
}

// ── SystemParam groups (Bevy 16-param limit) ──────────────────────────────────

#[derive(SystemParam)]
pub struct SaveParamsA<'w, 's> {
    pub gt: Res<'w, GameTime>,
    pub ms: Res<'w, Milestones>,
    pub rating: Res<'w, LifeRating>,
    pub player_q: Query<'w, 's, (&'static PlayerStats, &'static Skills, &'static WorkStreak, &'static HousingTier, &'static Inventory), With<LocalPlayer>>,
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
    pub player_q: Query<'w, 's, (&'static mut PlayerStats, &'static mut Skills, &'static mut WorkStreak, &'static mut HousingTier, &'static mut Inventory), With<LocalPlayer>>,
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

/// Load raw save data from storage. Returns `None` if no save or parse error.
pub fn load_save_data() -> Option<SaveData> {
    let contents = read_save_text()?;
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
    mut notif: ResMut<Notification>,
) {
    if events.is_empty() {
        return;
    }
    events.clear();

    let Ok((stats, skills, streak, housing, inv)) = a.player_q.get_single() else {
        return;
    };

    let mut npc_friendship = [0f32; 6];
    for (entity, npc_id) in &npc_q {
        if npc_id.0 < 6 {
            npc_friendship[npc_id.0] = friendship.levels.get(&entity).copied().unwrap_or(0.);
        }
    }

    let data = SaveData {
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
        day: a.gt.day,
        skill_cooking: skills.cooking,
        skill_career: skills.career,
        skill_fitness: skills.fitness,
        skill_social: skills.social,
        streak_days: streak.days,
        housing: u8::from(&*housing),
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
        ms_festival_goer: a.ms.festival_goer,
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

    let Ok((mut stats, mut skills, mut streak, mut housing, mut inv)) =
        a.player_q.get_single_mut()
    else {
        return;
    };

    // ── PlayerStats ───────────────────────────────────────────────────────────
    stats.energy = data.energy;
    stats.hunger = data.hunger;
    stats.happiness = data.happiness;
    stats.health = data.health;
    stats.stress = data.stress;
    stats.sleep_debt = data.sleep_debt;
    stats.money = data.money;
    stats.savings = data.savings;
    stats.loan = data.loan;
    stats.meals = data.meals;
    stats.unpaid_rent_days = data.unpaid_rent_days;
    stats.meditation_buff = data.meditation_buff;

    // ── GameTime ──────────────────────────────────────────────────────────────
    a.gt.day = data.day;
    a.gt.prev_day = data.day; // prevent false new-day trigger
    a.gt.hours = 8.; // start each loaded session in the morning

    // ── Skills ────────────────────────────────────────────────────────────────
    skills.cooking = data.skill_cooking;
    skills.career = data.skill_career;
    skills.fitness = data.skill_fitness;
    skills.social = data.skill_social;

    // ── WorkStreak ────────────────────────────────────────────────────────────
    streak.days = data.streak_days;

    // ── HousingTier ───────────────────────────────────────────────────────────
    *housing = HousingTier::from(data.housing);

    // ── Inventory ─────────────────────────────────────────────────────────────
    inv.coffee = data.inv_coffee;
    inv.vitamins = data.inv_vitamins;
    inv.books = data.inv_books;
    inv.coffee_age = data.inv_coffee_age;
    inv.ingredient = data.inv_ingredient;
    inv.gift_box = data.inv_gift_box;
    inv.smoothie = data.inv_smoothie;

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
    a.ms.quest_master = data.ms_quest_master;
    a.ms.master_chef = data.ms_master_chef;
    a.ms.gift_giver = data.ms_gift_giver;
    a.ms.popular = data.ms_popular;
    a.ms.crisis_survivor = data.ms_crisis_survivor;

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
    b.conds.hospitalized = data.hospitalized;
    b.conds.hospital_timer = data.hospital_timer;
    b.conds.mental_fatigue = data.mental_fatigue;
    b.conds.high_stress_days = data.high_stress_days;
    b.conds.low_stress_days = data.low_stress_days;

    // ── Investment ────────────────────────────────────────────────────────────
    b.invest.amount = data.invest_amount;
    b.invest.risk = data.invest_risk;
    b.invest.daily_return_rate = data.invest_rate;
    b.invest.total_return = data.invest_total_return;

    // ── Reputation ────────────────────────────────────────────────────────────
    b.rep.score = data.rep_score;

    // ── Transport ─────────────────────────────────────────────────────────────
    b.transport.kind = TransportKind::from(data.transport);
    b.transport.work_uses = data.transport_work_uses;
    b.transport.maintenance_due = data.transport_maintenance_due;
    if b.transport.kind == TransportKind::Car {
        for mut vis in &mut vehicle_q {
            *vis = Visibility::Visible;
        }
    }

    // ── Pet ───────────────────────────────────────────────────────────────────
    b.pet.has_pet = data.has_pet;
    b.pet.hunger = data.pet_hunger;
    b.pet.name = data.pet_name;
    b.pet.fed_today = false;
    b.pet.kind = PetKind::from(data.pet_kind);

    // ── SocialEvents ──────────────────────────────────────────────────────────
    b.social.parties_thrown = data.parties_thrown;

    // ── Season (derived from day) ─────────────────────────────────────────────
    b.season.current = SeasonKind::from_day(data.day);

    // ── GameState ─────────────────────────────────────────────────────────────
    b.gs.days_survived = data.days_survived;

    // ── Quests ────────────────────────────────────────────────────────────────
    b.quest_board.completed_total = data.quests_completed_total;
    b.gs.total_gifts = data.total_gifts;
    b.gs.total_crafted = data.total_crafted;
    b.gs.total_quests = data.total_quests;

    // ── Crisis ────────────────────────────────────────────────────────────────
    b.crisis.active = CrisisKind::from_u8(data.crisis_kind);
    b.crisis.days_left = data.crisis_days_left;
    b.crisis.crises_survived = data.crises_survived;
    b.crisis.last_crisis_day = data.crisis_last_day;
    b.crisis.has_insurance = data.has_insurance;
    b.crisis.insurance_days = data.insurance_days;

    // ── Festival ──────────────────────────────────────────────────────────────
    b.festival.tokens = data.festival_tokens;
    b.festival.spring_attended = data.festival_spring;
    b.festival.summer_attended = data.festival_summer;
    b.festival.autumn_attended = data.festival_autumn;
    b.festival.winter_attended = data.festival_winter;
    b.festival.festivals_total = data.festivals_total;
    a.ms.festival_goer = data.ms_festival_goer;

    // ── NPC Friendship ────────────────────────────────────────────────────────
    friendship.levels.clear();
    for (entity, npc_id) in &npc_q {
        if npc_id.0 < 6 {
            friendship
                .levels
                .insert(entity, data.npc_friendship[npc_id.0]);
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
        With<Player>,
    >,
    mut goal: ResMut<DailyGoal>,
    mut lightning: ResMut<LightningTimer>,
    npc_q: Query<(Entity, &NpcId)>,
) {
    if *start_kind == GameStartKind::Resume {
        return;
    }

    if let Ok((mut stats, mut skills, mut streak, mut housing, mut inv)) =
        a.player_q.get_single_mut()
    {
        *stats = PlayerStats::default();
        *skills = Skills::default();
        *streak = WorkStreak::default();
        *housing = HousingTier::default();
        *inv = Inventory::default();
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
    if let Ok((mut movement, mut vehicle_state, mut bank_input, mut action_prompt)) =
        player_state_q.get_single_mut()
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

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::SaveData;

    // Adding a new SaveData field checklist:
    // 1. Add field + #[serde(default)] to SaveData struct
    // 2. Serialize in handle_save (data.field = resource.field)
    // 3. Deserialize in apply_save_data (resource.field = data.field)
    // 4. Add field to sample_save() below (compile error if missed)
    fn sample_save() -> SaveData {
        SaveData {
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
            day: 5,
            skill_cooking: 1.5,
            skill_career: 2.0,
            skill_fitness: 0.5,
            skill_social: 1.0,
            streak_days: 3,
            housing: 1,
            inv_coffee: 2,
            inv_vitamins: 1,
            inv_books: 0,
            inv_coffee_age: 2,
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
            quests_completed_total: 0,
            total_gifts: 0,
            total_crafted: 0,
            total_quests: 0,
            inv_ingredient: 0,
            inv_gift_box: 0,
            inv_smoothie: 0,
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
            ms_festival_goer: false,
        }
    }

    #[test]
    fn save_data_round_trips_via_json() {
        let original = sample_save();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize");

        assert!((original.energy - restored.energy).abs() < f32::EPSILON);
        assert!((original.money - restored.money).abs() < f32::EPSILON);
        assert_eq!(original.day, restored.day);
        assert_eq!(original.housing, restored.housing);
        assert_eq!(original.ms_exec, restored.ms_exec);
        assert_eq!(original.ms_debt_free, restored.ms_debt_free);
        assert_eq!(original.transport, restored.transport);
        assert!((original.npc_friendship[1] - restored.npc_friendship[1]).abs() < f32::EPSILON);
    }

    #[test]
    fn save_data_default_is_valid_json() {
        let data = SaveData::default();
        let json = serde_json::to_string(&data).expect("serialize default");
        let _: SaveData = serde_json::from_str(&json).expect("deserialize default");
    }

    #[test]
    fn penthouse_housing_round_trips() {
        let mut data = sample_save();
        data.housing = 3; // Penthouse
        let json = serde_json::to_string(&data).expect("serialize");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.housing, 3);
    }

    #[test]
    fn pet_kind_fish_round_trips() {
        let mut data = sample_save();
        data.has_pet = true;
        data.pet_kind = 2; // Fish
        data.pet_name = "Nemo".to_string();
        data.pet_hunger = 15.;
        let json = serde_json::to_string(&data).expect("serialize");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.pet_kind, 2);
        assert_eq!(restored.pet_name, "Nemo");
        assert!((restored.pet_hunger - 15.).abs() < f32::EPSILON);
    }

    #[test]
    fn friendship_boundary_values_round_trip() {
        let mut data = sample_save();
        data.npc_friendship = [0., 5., 0., 5., 2.5, 0.];
        let json = serde_json::to_string(&data).expect("serialize");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize");
        assert!((restored.npc_friendship[0] - 0.).abs() < f32::EPSILON);
        assert!((restored.npc_friendship[1] - 5.).abs() < f32::EPSILON);
        assert!((restored.npc_friendship[4] - 2.5).abs() < f32::EPSILON);
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
        data.ms_saved_100 = true;
        data.ms_exec = true;
        data.ms_best_friend = true;
        data.ms_streak_7 = true;
        data.ms_rating_a = true;
        data.ms_debt_free = true;
        data.ms_penthouse = true;
        data.ms_investor = true;
        data.ms_hobbyist = true;
        data.ms_famous = true;
        data.ms_scholar = true;
        data.ms_pet_owner = true;
        data.ms_party_animal = true;
        data.ms_commuter = true;
        data.ms_all_seasons = true;
        let json = serde_json::to_string(&data).expect("serialize");
        let restored: SaveData = serde_json::from_str(&json).expect("deserialize");
        assert!(
            restored.ms_saved_100
                && restored.ms_exec
                && restored.ms_penthouse
                && restored.ms_all_seasons
                && restored.ms_party_animal
        );
    }
}
