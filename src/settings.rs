use bevy::prelude::*;
use serde::{Deserialize, Serialize};

const SETTINGS_PATH: &str = "config.toml";

/// Persisted game settings. Loaded at startup from `config.toml`; defaults written
/// if the file is missing or malformed.
#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
pub struct GameSettings {
    pub window_width: f32,
    pub window_height: f32,
    /// Master volume in [0.0, 1.0].
    pub master_volume: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            window_width: 1150.,
            window_height: 800.,
            master_volume: 0.8,
        }
    }
}

impl GameSettings {
    /// Load settings from `config.toml`, falling back to defaults on any error.
    /// Writes defaults to disk if the file was missing or unparseable.
    pub fn load_or_default() -> Self {
        if let Ok(contents) = std::fs::read_to_string(SETTINGS_PATH) {
            if let Ok(settings) = toml::from_str::<GameSettings>(&contents) {
                return settings;
            }
        }
        let defaults = GameSettings::default();
        defaults.save();
        defaults
    }

    /// Persist current settings to `config.toml`.
    pub fn save(&self) {
        match toml::to_string_pretty(self) {
            Ok(toml_str) => {
                if let Err(e) = std::fs::write(SETTINGS_PATH, toml_str) {
                    eprintln!("[settings] Failed to write config.toml: {e}");
                }
            }
            Err(e) => eprintln!("[settings] Failed to serialize settings: {e}"),
        }
    }
}

/// Apply settings that can only be set at startup (window size is handled by
/// `WindowPlugin` before this runs, so we just log it). Applies master volume
/// via `GlobalVolume`.
pub fn apply_settings(settings: Res<GameSettings>, mut global_vol: ResMut<GlobalVolume>) {
    global_vol.volume = bevy::audio::Volume::new(settings.master_volume);
}
