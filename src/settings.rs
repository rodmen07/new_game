use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
const SETTINGS_PATH: &str = "config.toml";
#[cfg(target_arch = "wasm32")]
const SETTINGS_STORAGE_KEY: &str = "new_game.settings.toml";

#[cfg(target_arch = "wasm32")]
fn browser_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn read_settings_text() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        browser_storage()?
            .get_item(SETTINGS_STORAGE_KEY)
            .ok()
            .flatten()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(SETTINGS_PATH).ok()
    }
}

fn write_settings_text(contents: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) = browser_storage() else {
            eprintln!("[settings] Browser localStorage is unavailable");
            return false;
        };
        if let Err(e) = storage.set_item(SETTINGS_STORAGE_KEY, contents) {
            eprintln!("[settings] Failed to write browser storage: {e:?}");
            return false;
        }
        true
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Err(e) = std::fs::write(SETTINGS_PATH, contents) {
            eprintln!("[settings] Failed to write config.toml: {e}");
            false
        } else {
            true
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Normal,
    #[default]
    Hard,
}

impl Difficulty {
    pub fn label(&self) -> &str {
        match self {
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
        }
    }

    pub fn hunger_mult(&self) -> f32 {
        match self {
            Self::Easy => 0.90,
            Self::Normal => 1.0,
            Self::Hard => 1.15,
        }
    }

    pub fn energy_decay_mult(&self) -> f32 {
        match self {
            Self::Easy => 0.85,
            Self::Normal => 1.0,
            Self::Hard => 1.20,
        }
    }

    pub fn health_drain_mult(&self) -> f32 {
        match self {
            Self::Easy => 0.85,
            Self::Normal => 1.0,
            Self::Hard => 1.20,
        }
    }

    pub fn economy_mult(&self) -> f32 {
        match self {
            Self::Easy => 1.15,
            Self::Normal => 1.0,
            Self::Hard => 0.90,
        }
    }

    pub fn rent_mult(&self) -> f32 {
        match self {
            Self::Easy => 0.90,
            Self::Normal => 1.0,
            Self::Hard => 1.15,
        }
    }

    pub fn loan_interest_mult(&self) -> f32 {
        match self {
            Self::Easy => 0.75,
            Self::Normal => 1.0,
            Self::Hard => 1.25,
        }
    }
}

/// Persisted game settings. Loaded at startup from config.toml on native builds
/// and browser localStorage on web builds; defaults are written back if missing
/// or malformed.
#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct GameSettings {
    pub window_width: f32,
    pub window_height: f32,
    /// Master volume in [0.0, 1.0].
    pub master_volume: f32,
    pub difficulty: Difficulty,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            window_width: 1150.,
            window_height: 800.,
            master_volume: 0.8,
            difficulty: Difficulty::Hard,
        }
    }
}

impl GameSettings {
    /// Load settings from storage, falling back to defaults on any error.
    /// Writes defaults back to the active platform store if missing or unparseable.
    pub fn load_or_default() -> Self {
        if let Some(contents) = read_settings_text()
            && let Ok(settings) = toml::from_str::<GameSettings>(&contents)
        {
            return settings;
        }
        let defaults = GameSettings::default();
        defaults.save();
        defaults
    }

    /// Persist current settings to the active platform store. Returns `true` on success.
    pub fn save(&self) -> bool {
        match toml::to_string_pretty(self) {
            Ok(toml_str) => write_settings_text(&toml_str),
            Err(e) => {
                eprintln!("[settings] Failed to serialize settings: {e}");
                false
            }
        }
    }
}

/// Apply settings that can only be set at startup (window size is handled by
/// `WindowPlugin` before this runs, so we just log it). Applies master volume
/// via `GlobalVolume`.
pub fn apply_settings(settings: Res<GameSettings>, mut global_vol: ResMut<GlobalVolume>) {
    global_vol.volume = bevy::audio::Volume::new(settings.master_volume);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_multipliers_are_ordered() {
        assert!(Difficulty::Easy.energy_decay_mult() < Difficulty::Normal.energy_decay_mult());
        assert!(Difficulty::Hard.energy_decay_mult() > Difficulty::Normal.energy_decay_mult());
        assert!(Difficulty::Easy.economy_mult() > Difficulty::Normal.economy_mult());
        assert!(Difficulty::Hard.economy_mult() < Difficulty::Normal.economy_mult());
    }

    #[test]
    fn default_settings_use_hard_difficulty() {
        let settings = GameSettings::default();
        assert_eq!(settings.difficulty, Difficulty::Hard);
    }
}
