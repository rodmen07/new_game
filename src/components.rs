use bevy::prelude::*;

#[derive(Component)]
pub struct Player;
/// Marks the player entity controlled by the local keyboard/gamepad.
/// In single-player this is always present on the sole `Player` entity.
/// In multiplayer, remote players have `Player` but not `LocalPlayer`.
#[derive(Component)]
pub struct LocalPlayer;
/// Stable identity for a player across save/load and network sessions.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u32);
#[derive(Component)]
pub struct MainCamera;
#[derive(Component)]
pub struct Vehicle;

#[derive(Clone, Copy, PartialEq)]
pub enum PetKind {
    Dog,
    Cat,
    Fish,
}
impl From<PetKind> for u8 {
    fn from(p: PetKind) -> u8 {
        match p {
            PetKind::Dog => 0,
            PetKind::Cat => 1,
            PetKind::Fish => 2,
        }
    }
}
impl From<&PetKind> for u8 {
    fn from(p: &PetKind) -> u8 {
        match p {
            PetKind::Dog => 0,
            PetKind::Cat => 1,
            PetKind::Fish => 2,
        }
    }
}
impl From<u8> for PetKind {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::Cat,
            2 => Self::Fish,
            _ => Self::Dog,
        }
    }
}
impl PetKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Dog => "Buddy",
            Self::Cat => "Whiskers",
            Self::Fish => "Finn",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dog => "Dog",
            Self::Cat => "Cat",
            Self::Fish => "Fish",
        }
    }
}

#[derive(Component, Clone, PartialEq)]
pub enum ActionKind {
    Sleep,
    Eat,
    Work,
    Freelance,
    Shop,
    Relax,
    Shower,
    Chat,
    Exercise,
    Meditate,
    Bank,
    UseItem(ItemKind),
    Hobby(HobbyKind),
    StudyCourse,
    FeedPet,
    ThrowParty,
    BuyTransport,
    GymSession,
    Cafe,
    Clinic,
    EnterVehicle,
    AdoptPet(PetKind),
    SleepRough,
    Craft,
    // New collective-building actions
    RentUnit(u32),
    GasUp,
    RepairVehicle,
    DentalVisit,
    EyeExam,
    ComputerLab,
    PrintShop,
}

// ── Building classification ────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BuildingKind {
    /// Houses one household (HOME, SUBURBS).
    Individual,
    /// Serves the whole community.
    Collective,
}

#[derive(Component, Clone)]
#[allow(dead_code)]
pub struct Building {
    pub name: &'static str,
    pub kind: BuildingKind,
}

/// Marks an apartment unit inside the APARTMENTS building.
#[derive(Component, Clone)]
#[allow(dead_code)]
pub struct ApartmentUnit {
    pub unit_id: u32,
    pub owner: Option<PlayerId>,
}

#[derive(Component, Clone, PartialEq)]
pub enum ItemKind {
    Coffee,
    Vitamins,
    Books,
    #[allow(dead_code)]
    Ingredient,
    #[allow(dead_code)]
    GiftBox,
    Smoothie,
}

#[derive(Clone, PartialEq)]
pub enum HobbyKind {
    Painting,
    Gaming,
    Music,
}

#[derive(Component)]
pub struct Interactable {
    pub action: ActionKind,
    pub prompt: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum NpcPersonality {
    Neutral,     // no special effect
    Cheerful,    // chat: +50% happiness; gift: +50% happiness
    Wise,        // chat: +50% social XP; gift: +15 health
    Influential, // chat: +3 rep; gift: +15 rep
}

#[derive(Component)]
pub struct Npc {
    pub name: String,
    pub wander_timer: f32,
    pub target: Vec2,
    pub zone_center: Vec2,
    pub zone_half: f32,
    pub rng: u64,
    pub velocity: Vec2,
    pub personality: NpcPersonality,
    pub home_zone: Vec2, // night (21-06): wander here
    pub work_zone: Vec2, // work hours (09-17): wander here
}

#[derive(Component)]
pub struct NpcLabel(pub Entity);

#[derive(Component)]
pub enum HudLabel {
    Time,
    Money,
    Prompt,
    Warning,
    Skills,
    Goal,
    Story,
    Notification,
    Mood,
    Friendship,
    Rent,
    Rating,
    Inventory,
    Streak,
    Housing,
    Milestones,
    Weather,
    Hobbies,
    Conditions,
    Reputation,
    Season,
    Pet,
    Transport,
    Quest,
}

#[derive(Component)]
pub enum HudBar {
    Energy,
    Hunger,
    Happiness,
    Health,
    Stress,
}

#[derive(Component)]
pub struct DayNightOverlay;
#[derive(Component)]
pub struct InteractHighlight;
#[derive(Component)]
pub struct ObjectSize(pub Vec2);
#[derive(Component)]
pub struct PlayerIndicator;
/// Smoothed display value for a stat bar (0–100). Lerps toward `target`
/// each frame so bars drain/fill visibly instead of jumping instantly.
#[derive(Component, Default)]
pub struct BarSmooth {
    pub displayed: f32,
    pub target: f32,
}
/// Marks the top-center notification container node so the slide-in animator
/// can locate it by query.
#[derive(Component)]
pub struct NotifContainer;
#[derive(Component)]
pub struct WeatherDrop {
    pub vel: Vec2,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub base_color: [f32; 4],
}

// ── Quest Types ───────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub enum QuestKind {
    FetchItem(ItemKind, u32),    // bring N of an item
    DoActivity(ActionKind, u32), // perform action N times
    EarnMoney(f32),              // earn at least $X in a day
    CraftItem(u32),              // craft N items total
}
impl QuestKind {
    pub fn description(&self) -> String {
        match self {
            Self::FetchItem(item, n) => {
                let name = match item {
                    ItemKind::Coffee => "coffee",
                    ItemKind::Vitamins => "vitamins",
                    ItemKind::Books => "books",
                    ItemKind::Ingredient => "ingredients",
                    ItemKind::GiftBox => "gift boxes",
                    ItemKind::Smoothie => "smoothies",
                };
                format!("Bring me {} {}", n, name)
            }
            Self::DoActivity(action, n) => {
                let name = match action {
                    ActionKind::Work => "work shift",
                    ActionKind::Exercise => "exercise session",
                    ActionKind::Chat => "chat",
                    ActionKind::Meditate => "meditation",
                    ActionKind::Eat => "meal",
                    ActionKind::Hobby(_) => "hobby session",
                    ActionKind::GymSession => "gym session",
                    _ => "activity",
                };
                if *n > 1 {
                    format!("Do {} {}s", n, name)
                } else {
                    format!("Do a {}", name)
                }
            }
            Self::EarnMoney(amt) => format!("Earn ${:.0} today", amt),
            Self::CraftItem(n) => {
                if *n > 1 {
                    format!("Craft {} items", n)
                } else {
                    "Craft an item".to_string()
                }
            }
        }
    }
}

/// Axis-aligned bounding box for collision. Stores half-extents (w/2, h/2).
#[derive(Component)]
pub struct Collider(pub Vec2);

/// Tags child sprites that make up a humanoid composite figure.
#[derive(Component, PartialEq)]
pub enum BodyPart {
    LeftLeg,
    RightLeg,
    LeftFoot,
    RightFoot,
    Body,
    Head,
    Hair,
}

/// Stable NPC index used to persist friendship across save/load.
/// Alex=0, Sam=1, Mia=2, Jordan=3, Taylor=4, Casey=5.
#[derive(Component, Clone, Copy)]
pub struct NpcId(pub usize);
