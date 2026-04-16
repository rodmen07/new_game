use bevy::prelude::*;

#[derive(Component)] pub struct Player;
#[derive(Component)] pub struct MainCamera;

#[derive(Component, Clone)]
pub enum ActionKind {
    Sleep, Eat, Work, Freelance, Shop, Relax, Shower, Chat, Exercise, Meditate, Bank,
    UseItem(ItemKind), Hobby(HobbyKind),
    StudyCourse, FeedPet, ThrowParty, BuyTransport,
}

#[derive(Component, Clone, PartialEq)]
pub enum ItemKind { Coffee, Vitamins, Books }

#[derive(Clone, PartialEq)]
pub enum HobbyKind { Painting, Gaming, Music }
impl HobbyKind {
    pub fn label(&self) -> &str { match self { Self::Painting=>"Painting", Self::Gaming=>"Gaming", Self::Music=>"Music" } }
}

#[derive(Component)]
pub struct Interactable { pub action: ActionKind, pub prompt: String }

#[derive(Component)]
pub struct Npc {
    pub name: String, pub wander_timer: f32, pub target: Vec2,
    pub zone_center: Vec2, pub zone_half: f32, pub rng: u64,
    pub velocity: Vec2,
}

#[derive(Component)]
pub struct NpcLabel(pub Entity);

#[derive(Component)]
pub enum HudLabel {
    Time, Money, Prompt, Warning, Skills, Goal, Notification,
    Mood, Friendship, Rent, Rating, Inventory, Streak, Housing, Milestones,
    Weather, Hobbies, Conditions, Reputation,
    Season, Pet, Transport,
}

#[derive(Component)]
pub enum HudBar { Energy, Hunger, Happiness, Health, Stress }

#[derive(Component)] pub struct DayNightOverlay;
#[derive(Component)] pub struct InteractHighlight;
#[derive(Component)] pub struct ObjectSize(pub Vec2);
#[derive(Component)] pub struct PlayerIndicator;
/// Axis-aligned bounding box for collision. Stores half-extents (w/2, h/2).
#[derive(Component)] pub struct Collider(pub Vec2);

/// Tags child sprites that make up a humanoid composite figure.
#[derive(Component, PartialEq)]
pub enum BodyPart { LeftLeg, RightLeg, LeftFoot, RightFoot, Body, Head, Hair }

/// Stable NPC index used to persist friendship across save/load.
/// Alex = 0, Sam = 1, Mia = 2.
#[derive(Component, Clone, Copy)]
pub struct NpcId(pub usize);
