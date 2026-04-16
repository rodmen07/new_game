mod components;
mod constants;
mod resources;
mod save;
mod settings;
mod setup;
mod systems;

use bevy::prelude::*;
use resources::*;
use save::{PendingLoad, SaveRequest, apply_save_data, handle_save};
use settings::{GameSettings, apply_settings};
use setup::setup;
use systems::*;

fn main() {
    let settings = GameSettings::load_or_default();
    let (w, h) = (settings.window_width, settings.window_height);

    App::new()
        .insert_resource(settings)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Everyday Life Simulator".into(),
                resolution: (w, h).into(),
                ..default()
            }),
            ..default()
        }))
        // ── Game resources ────────────────────────────────────────────────────
        .init_resource::<PlayerMovement>()
        .init_resource::<PlayerStats>()
        .init_resource::<Inventory>()
        .init_resource::<GameTime>()
        .init_resource::<NearbyInteractable>()
        .init_resource::<Skills>()
        .init_resource::<WorkStreak>()
        .init_resource::<HousingTier>()
        .init_resource::<NpcFriendship>()
        .init_resource::<Notification>()
        .init_resource::<LifeRating>()
        .init_resource::<Milestones>()
        .init_resource::<DailyGoal>()
        .init_resource::<GameState>()
        .init_resource::<WeatherKind>()
        .init_resource::<Hobbies>()
        .init_resource::<Conditions>()
        .init_resource::<Investment>()
        .init_resource::<Reputation>()
        .init_resource::<Transport>()
        .init_resource::<Pet>()
        .init_resource::<SocialEvents>()
        .init_resource::<Season>()
        // ── Save / load ───────────────────────────────────────────────────────
        .init_resource::<PendingLoad>()
        .add_event::<SaveRequest>()
        // ── Systems ───────────────────────────────────────────────────────────
        .add_systems(Startup, (apply_settings, setup, apply_save_data).chain())
        .add_systems(Update, camera_zoom)
        .add_systems(Update, (
            tick_time, on_new_day, decay_stats, degrade_health, check_critical,
            player_movement, resolve_collisions, player_visuals,
            npc_wander, npc_visuals, update_npc_labels, update_npc_prompts,
        ).chain())
        .add_systems(Update, (
            detect_nearby, update_highlight, handle_interaction,
            check_daily_goal, check_milestones,
        ).chain().after(player_movement))
        .add_systems(Update, (
            spawn_dash_particles, update_particles,
            camera_follow, tick_notification, update_hud, update_day_night,
        ).chain().after(player_visuals))
        .add_systems(Update, handle_save.after(on_new_day))
        .run();
}
