use bevy::{prelude::*, ecs::system::SystemParam, utils::HashMap};
use crate::save::SaveRequest;

#[derive(Resource, Default)]
pub struct NearbyInteractable { pub entity: Option<Entity>, pub prompt: String }

// ── PlayerMovement ────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct PlayerMovement {
    pub velocity: Vec2,
    pub dash_cooldown: f32,
    pub dashing: bool,
    pub dash_timer: f32,
    pub dash_dir: Vec2,
    pub sprint_energy_timer: f32,
    pub base_zoom: f32,
}
impl Default for PlayerMovement {
    fn default() -> Self {
        Self { velocity: Vec2::ZERO, dash_cooldown: 0., dashing: false, dash_timer: 0., dash_dir: Vec2::ZERO, sprint_energy_timer: 0., base_zoom: 1.0 }
    }
}

// ── PlayerStats ───────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct PlayerStats {
    pub energy: f32, pub hunger: f32, pub happiness: f32, pub health: f32, pub stress: f32,
    pub sleep_debt: f32, pub money: f32, pub savings: f32, pub loan: f32, pub meals: u32,
    pub cooldown: f32, pub critical_timer: f32, pub unpaid_rent_days: u32, pub meditation_buff: f32,
}
impl Default for PlayerStats {
    fn default() -> Self {
        Self { energy:80., hunger:20., happiness:70., health:100., stress:10.,
               sleep_debt:0., money:100., savings:0., loan:0.,
               meals:2, cooldown:0., critical_timer:0., unpaid_rent_days:0, meditation_buff:0. }
    }
}
impl PlayerStats {
    pub fn max_energy(&self) -> f32 {
        if self.sleep_debt > 16. { 60. } else if self.sleep_debt > 8. { 80. } else { 100. }
    }
    pub fn stress_work_mult(&self) -> f32 {
        if self.stress > 75. { 0.50 } else if self.stress > 50. { 0.85 } else { 1.0 }
    }
    pub fn loan_penalty(&self) -> f32 { if self.loan > 300. { 0.90 } else { 1.0 } }
}

#[derive(Resource, Default)]
pub struct Inventory { pub coffee: u32, pub vitamins: u32, pub books: u32 }

// ── GameTime ──────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GameTime { pub hours: f32, pub day: u32, pub prev_day: u32 }
impl Default for GameTime { fn default() -> Self { Self { hours:8., day:0, prev_day:0 } } }
impl GameTime {
    pub fn display(&self) -> String {
        let h = self.hours as u32 % 24;
        let m = (self.hours.fract() * 60.) as u32;
        let (ampm, h12) = if h < 12 { ("AM", if h==0 {12} else {h}) }
                          else       { ("PM", if h==12{12} else {h-12}) };
        format!("Day {}  {:02}:{:02} {}  ({})", self.day+1, h12, m, ampm, self.day_name())
    }
    pub fn is_night(&self)   -> bool { self.hours >= 21. || self.hours < 6. }
    pub fn is_weekend(&self) -> bool { self.day % 7 >= 5 }
    pub fn day_name(&self)   -> &str { match self.day%7 { 0=>"Mon",1=>"Tue",2=>"Wed",3=>"Thu",4=>"Fri",5=>"Sat",_=>"Sun" } }
    pub fn work_time_tag(&self) -> (f32, &str) {
        let h = self.hours as u32;
        if h >= 6 && h < 10 { (1.15, " [Early Bird +15%]") }
        else if h >= 20     { (0.90, " [Late Night -10%]") }
        else                { (1.0, "") }
    }
    pub fn exercise_mult(&self) -> f32 { let h = self.hours as u32; if h >= 5 && h < 9 { 1.25 } else { 1.0 } }
    pub fn is_breakfast(&self)  -> bool { let h = self.hours as u32; h >= 6 && h < 9 }
}

// ── Skills ────────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct Skills { pub cooking: f32, pub career: f32, pub fitness: f32, pub social: f32 }
impl Skills {
    pub fn cooking_bonus(&self) -> f32 { 1. + self.cooking * 0.10 }
    pub fn career_bonus(&self)  -> f32 { 1. + self.career  * 0.12 }
    pub fn social_bonus(&self)  -> f32 { 1. + self.social  * 0.10 }
    pub fn fitness_bonus(&self) -> f32 { 1. + self.fitness * 0.05 }
    pub fn career_rank(&self)   -> &str {
        if self.career >= 5.0 { "Executive" } else if self.career >= 2.5 { "Senior" } else { "Junior" }
    }
    pub fn work_pay(&self, streak: u32) -> f32 {
        let base = if self.career >= 5.0 { 70. } else if self.career >= 2.5 { 45. } else { 30. };
        let s = if streak >= 7 { 1.35 } else if streak >= 5 { 1.20 } else if streak >= 3 { 1.10 } else { 1.0 };
        base * self.career_bonus() * s
    }
}

// ── WorkStreak ────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct WorkStreak { pub days: u32, pub worked_today: bool }

// ── HousingTier ───────────────────────────────────────────────────────────────

#[derive(Resource, Default, Clone, PartialEq)]
pub enum HousingTier { #[default] Apartment, Condo, Penthouse }
impl HousingTier {
    pub fn rent(&self)  -> f32  { match self { Self::Apartment=>20., Self::Condo=>35., Self::Penthouse=>60. } }
    pub fn label(&self) -> &str { match self { Self::Apartment=>"Apartment", Self::Condo=>"Condo", Self::Penthouse=>"Penthouse" } }
    pub fn upgrade_cost(&self) -> Option<f32> { match self { Self::Apartment=>Some(200.), Self::Condo=>Some(500.), Self::Penthouse=>None } }
    pub fn next(&self) -> Option<Self> { match self { Self::Apartment=>Some(Self::Condo), Self::Condo=>Some(Self::Penthouse), Self::Penthouse=>None } }
    pub fn sleep_energy(&self, night: bool) -> f32 {
        match self {
            Self::Apartment => if night { 70. } else { 35. },
            Self::Condo     => if night { 85. } else { 45. },
            Self::Penthouse => if night { 100. } else { 55. },
        }
    }
    pub fn night_health(&self)  -> f32 { match self { Self::Condo=>5., Self::Penthouse=>10., _=>0. } }
    pub fn morning_hap(&self)   -> f32 { match self { Self::Penthouse=>8., Self::Condo=>3., _=>0. } }
}

// ── NpcFriendship ─────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct NpcFriendship { pub levels: HashMap<Entity, f32>, pub chatted_today: HashMap<Entity, bool> }

// ── Misc Resources ────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct Notification { pub message: String, pub timer: f32 }

#[derive(Resource, Default)]
pub struct LifeRating { pub score: f32, pub days: u32 }
impl LifeRating {
    pub fn sample(&mut self, s: &PlayerStats, sk: &Skills) {
        let v = (s.energy*0.10 + (100.-s.hunger)*0.10 + s.happiness*0.20 + s.health*0.15
               + (100.-s.stress)*0.08 + (s.money.min(500.)/500.)*100.*0.08
               + (s.savings.min(1000.)/1000.)*100.*0.12
               + (sk.career+sk.cooking+sk.fitness+sk.social)/20.*100.*0.12
               - s.loan.min(500.)/500.*15.).max(0.).min(100.);
        self.score = (self.score*self.days as f32 + v) / (self.days+1) as f32;
        self.days += 1;
    }
    pub fn grade(&self) -> &str {
        match self.score as u32 {
            90..=100=>"S — Thriving",   75..=89=>"A — Flourishing",
            60..=74 =>"B — Comfortable",45..=59=>"C — Getting By",
            30..=44 =>"D — Struggling", _      =>"F — Crisis",
        }
    }
}

#[derive(Resource, Default)]
pub struct Milestones {
    pub saved_100: bool, pub exec: bool, pub best_friend: bool, pub streak_7: bool,
    pub rating_a: bool, pub debt_free: bool, pub penthouse: bool,
    pub investor: bool, pub hobbyist: bool, pub famous: bool,
    pub scholar: bool, pub pet_owner: bool, pub party_animal: bool,
    pub commuter: bool, pub all_seasons: bool,
}
impl Milestones {
    pub fn count(&self) -> u32 {
        [self.saved_100,self.exec,self.best_friend,self.streak_7,self.rating_a,
         self.debt_free,self.penthouse,self.investor,self.hobbyist,self.famous,
         self.scholar,self.pet_owner,self.party_animal,self.commuter,self.all_seasons]
            .iter().filter(|&&b|b).count() as u32
    }
    pub fn summary(&self) -> String {
        let mut v: Vec<&str> = vec![];
        if self.saved_100   { v.push("Saver"); }
        if self.exec        { v.push("Exec"); }
        if self.best_friend { v.push("BFF"); }
        if self.streak_7    { v.push("Streak7"); }
        if self.rating_a    { v.push("Flourishing"); }
        if self.debt_free   { v.push("Debt-Free"); }
        if self.penthouse   { v.push("Penthouse"); }
        if self.investor    { v.push("Investor"); }
        if self.hobbyist    { v.push("Hobbyist"); }
        if self.famous      { v.push("Famous"); }
        if self.scholar     { v.push("Scholar"); }
        if self.pet_owner   { v.push("Pet Owner"); }
        if self.party_animal{ v.push("Party Animal"); }
        if self.commuter    { v.push("Commuter"); }
        if self.all_seasons { v.push("All Seasons"); }
        if v.is_empty() { "None yet".into() } else { v.join("  ") }
    }
}

// ── Goal / Game State ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum GoalKind {
    EarnMoney, WorkTimes, MaintainHappy, EatTimes, ChatTimes,
    FriendNpc, SaveMoney, ExerciseTimes, LowerStress, BuildStreak,
    MasterHobby, EarnPassive, OutdoorWeather,
    StudyTimes, FeedPet, ThrowParty, OwnVehicle, SeasonalGoal,
}

#[derive(Resource)]
pub struct DailyGoal {
    pub kind: GoalKind, pub description: String, pub progress: f32, pub target: f32,
    pub reward_money: f32, pub reward_happiness: f32, pub completed: bool, pub failed: bool,
}
impl Default for DailyGoal {
    fn default() -> Self {
        Self { kind: GoalKind::EarnMoney, description: "Earn $50 today".into(),
               progress:0., target:50., reward_money:25., reward_happiness:0., completed:false, failed:false }
    }
}

#[derive(Resource, Default)]
pub struct GameState {
    pub days_survived: u32, pub money_earned_today: f32,
    pub work_today: u32, pub eat_today: u32, pub chat_today: u32, pub exercise_today: u32,
    pub hobby_today: u32, pub passive_income_today: f32, pub study_today: u32,
    pub outdoor_done_today: bool, pub high_stress_today: bool, pub high_hunger_today: bool,
}

// ── Mood ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Mood { Elated, Happy, Okay, Sad, Depressed }
impl Mood {
    pub fn from_happiness(h: f32) -> Self {
        match h as u32 { 80..=100=>Self::Elated,60..=79=>Self::Happy,40..=59=>Self::Okay,20..=39=>Self::Sad,_=>Self::Depressed }
    }
    pub fn label(&self)      -> &str { match self { Self::Elated=>"Elated",Self::Happy=>"Happy",Self::Okay=>"Okay",Self::Sad=>"Sad",Self::Depressed=>"Depressed" } }
    pub fn decay_mult(&self) -> f32  { match self { Self::Elated=>0.6,Self::Happy=>0.8,Self::Okay=>1.0,Self::Sad=>1.2,Self::Depressed=>1.5 } }
    pub fn work_mult(&self)  -> f32  { match self { Self::Elated=>1.2,Self::Happy=>1.1,Self::Okay=>1.0,Self::Sad=>0.9,Self::Depressed=>0.75 } }
}

// ── Weather ───────────────────────────────────────────────────────────────────

#[derive(Resource, Default, Clone, PartialEq)]
pub enum WeatherKind { #[default] Sunny, Cloudy, Rainy, Stormy }
impl WeatherKind {
    pub fn from_day(day: u32) -> Self {
        match day.wrapping_mul(1664525).wrapping_add(1013904223) % 10 {
            0..=3 => Self::Sunny,
            4..=6 => Self::Cloudy,
            7..=8 => Self::Rainy,
            _     => Self::Stormy,
        }
    }
    pub fn label(&self) -> &str {
        match self { Self::Sunny=>"Sunny", Self::Cloudy=>"Cloudy", Self::Rainy=>"Rainy", Self::Stormy=>"Stormy" }
    }
    pub fn energy_decay_mult(&self) -> f32 {
        match self { Self::Sunny=>0.9, Self::Cloudy=>1.0, Self::Rainy=>1.2, Self::Stormy=>1.5 }
    }
    pub fn outdoor_hap_bonus(&self) -> f32 {
        match self { Self::Sunny=>10., Self::Cloudy=>0., Self::Rainy=>-5., Self::Stormy=>0. }
    }
    pub fn is_stormy(&self) -> bool { *self == Self::Stormy }
    pub fn is_sunny(&self)  -> bool { *self == Self::Sunny  }
}

// ── Hobbies ───────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct Hobbies { pub painting: f32, pub gaming: f32, pub music: f32 }
impl Hobbies {
    pub fn passive_income(&self) -> f32 {
        let mut total = 0.;
        if self.painting >= 3. { total += 6.; }
        if self.gaming   >= 3. { total += 4.; }
        if self.music    >= 3. { total += 8.; }
        total
    }
    pub fn best(&self) -> f32 { self.painting.max(self.gaming).max(self.music) }
}

// ── Health Conditions ─────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct Conditions {
    pub burnout: bool, pub burnout_days: u32,
    pub malnourished: bool, pub malnourish_days: u32,
}
impl Conditions {
    pub fn work_pay_mult(&self) -> f32 { if self.burnout { 0.70 } else { 1.0 } }
}

// ── Investment ────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct Investment { pub amount: f32, pub risk: u8, pub daily_return_rate: f32, pub total_return: f32 }

// ── Reputation ────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct Reputation { pub score: f32 }
impl Reputation {
    pub fn chat_bonus(&self) -> f32 { 1.0 + self.score * 0.005 }
}

// ── Transport ─────────────────────────────────────────────────────────────────

#[derive(Default, Clone, PartialEq)]
pub enum TransportKind { #[default] Walk, Bike, Car }
impl TransportKind {
    pub fn work_bonus(&self) -> f32 { match self { Self::Walk=>1.0, Self::Bike=>1.10, Self::Car=>1.20 } }
    pub fn label(&self) -> &str { match self { Self::Walk=>"On foot", Self::Bike=>"Bicycle 🚲", Self::Car=>"Car 🚗" } }
    pub fn is_vehicle(&self) -> bool { *self != Self::Walk }
}

#[derive(Resource, Default)]
pub struct Transport { pub kind: TransportKind }

// ── Pet ───────────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct Pet { pub has_pet: bool, pub hunger: f32, pub fed_today: bool, pub name: String }
impl Default for Pet {
    fn default() -> Self { Self { has_pet: false, hunger: 0., fed_today: false, name: "Buddy".into() } }
}

// ── Social Events ─────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct SocialEvents { pub parties_thrown: u32, pub party_today: bool }

// ── Season ────────────────────────────────────────────────────────────────────

#[derive(Default, Clone, PartialEq)]
pub enum SeasonKind { #[default] Spring, Summer, Autumn, Winter }
impl SeasonKind {
    pub fn from_day(day: u32) -> Self {
        match (day / 30) % 4 { 0=>Self::Spring, 1=>Self::Summer, 2=>Self::Autumn, _=>Self::Winter }
    }
    pub fn label(&self) -> &str { match self { Self::Spring=>"🌸 Spring", Self::Summer=>"☀ Summer", Self::Autumn=>"🍂 Autumn", Self::Winter=>"❄ Winter" } }
    pub fn social_mult(&self)  -> f32 { match self { Self::Spring=>1.25, _=>1.0 } }
    pub fn outdoor_bonus(&self)-> f32 { match self { Self::Summer=>8., _=>0. } }
    pub fn passive_mult(&self) -> f32 { match self { Self::Autumn=>1.25, _=>1.0 } }
    pub fn indoor_bonus(&self) -> f32 { match self { Self::Winter=>5., _=>0. } }
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
pub struct Season { pub current: SeasonKind }

// ── SystemParam Groups (keeps update_hud and handle_interaction under 16 params) ──

#[derive(SystemParam)]
pub struct HudExtras<'w> {
    pub weather: Res<'w, WeatherKind>,
    pub hobbies: Res<'w, Hobbies>,
    pub conds:   Res<'w, Conditions>,
    pub rep:     Res<'w, Reputation>,
    pub pet:     Res<'w, Pet>,
    pub transport: Res<'w, Transport>,
    pub season:  Res<'w, Season>,
}

#[derive(SystemParam)]
pub struct InteractExtras<'w> {
    pub hobbies: ResMut<'w, Hobbies>,
    pub weather: Res<'w, WeatherKind>,
    pub rep:     ResMut<'w, Reputation>,
    pub invest:  ResMut<'w, Investment>,
    pub conds:   Res<'w, Conditions>,
    pub pet:     ResMut<'w, Pet>,
    pub transport: ResMut<'w, Transport>,
    pub social_events: ResMut<'w, SocialEvents>,
    pub season:  Res<'w, Season>,
}

#[derive(SystemParam)]
pub struct DayExtras<'w> {
    pub pet:          ResMut<'w, Pet>,
    pub social_events: ResMut<'w, SocialEvents>,
    pub season:       ResMut<'w, Season>,
    pub save_writer:  EventWriter<'w, SaveRequest>,
}