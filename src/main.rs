mod audio;
mod components;
mod constants;
mod menu;
mod resources;
mod save;
mod settings;
mod setup;
mod systems;

use audio::GameAudioPlugin;
use bevy::{asset::AssetPlugin, prelude::*};
use menu::{AppState, MenuPlugin, reset_start_kind};
use resources::*;
use save::{PendingLoad, SaveRequest, apply_save_data, handle_save, reset_game};
use settings::{GameSettings, apply_settings};
use setup::setup;
use std::path::PathBuf;
use systems::*;

fn asset_root() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .to_string_lossy()
        .into_owned()
}

fn main() {
    let settings = GameSettings::load_or_default();
    let (w, h) = (settings.window_width, settings.window_height);

    App::new()
        .insert_resource(settings)
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_root(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Everyday Life Simulator".to_string(),
                        resolution: (w, h).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(MenuPlugin)
        .add_plugins(GameAudioPlugin)
        // ── Game resources ────────────────────────────────────────────────────
        .init_resource::<PlayerStats>()
        .init_resource::<Inventory>()
        .init_resource::<GameTime>()
        .init_resource::<NearbyInteractable>()
        .init_resource::<Skills>()
        .init_resource::<WorkStreak>()
        .init_resource::<HousingTier>()
        .init_resource::<NpcFriendship>()
        .init_resource::<Notification>()
        .init_resource::<NarrativeState>()
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
        .init_resource::<QuestBoard>()
        .init_resource::<CrisisState>()
        .init_resource::<FestivalState>()
        .init_resource::<LightningTimer>()
        // ── Save / load ───────────────────────────────────────────────────────
        .init_resource::<PendingLoad>()
        .add_event::<SaveRequest>()
        // ── Systems ───────────────────────────────────────────────────────────
        .add_systems(Startup, (apply_settings, setup).chain())
        // Reset + apply save data every time we enter Playing (skipped on Resume).
        .add_systems(
            OnEnter(AppState::Playing),
            (reset_game, apply_save_data, reset_start_kind).chain(),
        )
        // Gameplay systems — only run in the Playing state.
        .add_systems(Update, camera_zoom.run_if(in_state(AppState::Playing)))
        .add_systems(Update, reveal_car_on_purchase.run_if(in_state(AppState::Playing)))
        .add_systems(
            Update,
            (
                tick_time,
                on_new_day,
                best_friend_perks,
                crisis_trigger_system,
                crisis_day_tick,
                festival_trigger_system,
                apply_eviction_teleport,
                decay_stats,
                degrade_health,
                check_critical,
                player_movement,
                car_movement,
                resolve_collisions,
                player_visuals,
                npc_wander,
                npc_visuals,
                update_npc_labels,
                update_npc_prompts,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            Update,
            (
                detect_nearby,
                update_highlight,
                handle_bank_input,
                handle_interaction,
                quest_offer_system,
                quest_progress_system,
                check_daily_goal,
                check_milestones,
                update_narrative,
            )
                .chain()
                .after(player_movement)
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            Update,
            (
                spawn_sprint_particles,
                update_particles,
                spawn_weather_particles,
                update_weather_drops,
                update_lightning,
                camera_follow,
                tick_notification,
                update_hud,
                update_day_night,
            )
                .chain()
                .after(player_visuals)
                .run_if(in_state(AppState::Playing)),
        )
        // handle_save is ungated: auto-save on new day (Playing) and on Main Menu (any state).
        .add_systems(Update, handle_save.after(on_new_day))
        .run();
}
