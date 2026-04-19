use crate::menu::AppState;
use crate::resources::{GameTime, LightningTimer, WeatherKind};
use bevy::audio::Volume;
use bevy::prelude::*;
use std::path::PathBuf;

// ── Event ─────────────────────────────────────────────────────────────────────

#[derive(Event, Clone, Copy)]
pub struct PlaySfx(pub SfxKind);

#[derive(Clone, Copy)]
pub enum SfxKind {
    Work,
    Eat,
    Sleep,
    Interact,
    Chime,
}

// ── Asset handles ─────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct AudioHandles {
    pub ambient: Option<Handle<AudioSource>>,
    pub chime: Option<Handle<AudioSource>>,
    pub work: Option<Handle<AudioSource>>,
    pub eat: Option<Handle<AudioSource>>,
    pub sleep: Option<Handle<AudioSource>>,
    pub interact: Option<Handle<AudioSource>>,
    pub bgm_day: Option<Handle<AudioSource>>,
    pub bgm_night: Option<Handle<AudioSource>>,
    pub rain_loop: Option<Handle<AudioSource>>,
    pub storm_loop: Option<Handle<AudioSource>>,
    pub thunder: Option<Handle<AudioSource>>,
}

// ── Markers ───────────────────────────────────────────────────────────────────

#[derive(Component)]
struct AmbientMusic;
#[derive(Component)]
struct MusicDay;
#[derive(Component)]
struct MusicNight;
#[derive(Component)]
struct WeatherAmbient;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioHandles>()
            .add_event::<PlaySfx>()
            .add_systems(Startup, load_audio_handles)
            .add_systems(OnEnter(AppState::Playing), start_ambient_if_needed)
            .add_systems(OnEnter(AppState::Menu), stop_ambient)
            .add_systems(
                Update,
                (play_sfx_events, update_bgm, update_weather_audio)
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

fn asset_disk_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(relative)
}

fn load_optional_audio(server: &AssetServer, relative: &str) -> Option<Handle<AudioSource>> {
    let disk_path = asset_disk_path(relative);
    if disk_path.exists() {
        Some(server.load(relative.to_string()))
    } else {
        warn!("[audio] Missing asset, skipping: {}", disk_path.display());
        None
    }
}

fn load_audio_handles(mut handles: ResMut<AudioHandles>, server: Res<AssetServer>) {
    handles.ambient = load_optional_audio(&server, "audio/ambient.ogg");
    handles.chime = load_optional_audio(&server, "audio/chime.ogg");
    handles.work = load_optional_audio(&server, "audio/work.ogg");
    handles.eat = load_optional_audio(&server, "audio/eat.ogg");
    handles.sleep = load_optional_audio(&server, "audio/sleep.ogg");
    handles.interact = load_optional_audio(&server, "audio/interact.ogg");
    handles.bgm_day = load_optional_audio(&server, "audio/bgm_day.ogg");
    handles.bgm_night = load_optional_audio(&server, "audio/bgm_night.ogg");
    handles.rain_loop = load_optional_audio(&server, "audio/rain_loop.wav");
    handles.storm_loop = load_optional_audio(&server, "audio/storm_loop.wav");
    handles.thunder = load_optional_audio(&server, "audio/thunder.wav");
}

fn start_ambient_if_needed(
    mut commands: Commands,
    handles: Res<AudioHandles>,
    existing: Query<(), With<AmbientMusic>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(ambient) = handles.ambient.clone() else {
        return;
    };
    commands.spawn((
        AmbientMusic,
        AudioPlayer::<AudioSource>(ambient),
        PlaybackSettings {
            volume: Volume::new(0.35),
            ..PlaybackSettings::LOOP
        },
    ));
}

fn stop_ambient(
    mut commands: Commands,
    ambient_q: Query<Entity, With<AmbientMusic>>,
    day_q: Query<Entity, With<MusicDay>>,
    night_q: Query<Entity, With<MusicNight>>,
    weather_q: Query<Entity, With<WeatherAmbient>>,
) {
    for e in ambient_q.iter().chain(day_q.iter()).chain(night_q.iter()).chain(weather_q.iter()) {
        commands.entity(e).despawn();
    }
}

fn update_bgm(
    mut commands: Commands,
    handles: Res<AudioHandles>,
    gt: Res<GameTime>,
    day_q: Query<Entity, With<MusicDay>>,
    night_q: Query<Entity, With<MusicNight>>,
) {
    let is_day = gt.hours >= 6. && gt.hours < 21.;

    if is_day {
        if !night_q.is_empty() {
            for e in &night_q {
                commands.entity(e).despawn();
            }
        }
        if day_q.is_empty()
            && let Some(src) = handles.bgm_day.clone()
        {
            commands.spawn((
                MusicDay,
                AudioPlayer::<AudioSource>(src),
                PlaybackSettings { volume: Volume::new(0.25), ..PlaybackSettings::LOOP },
            ));
        }
    } else {
        if !day_q.is_empty() {
            for e in &day_q {
                commands.entity(e).despawn();
            }
        }
        if night_q.is_empty()
            && let Some(src) = handles.bgm_night.clone()
        {
            commands.spawn((
                MusicNight,
                AudioPlayer::<AudioSource>(src),
                PlaybackSettings { volume: Volume::new(0.20), ..PlaybackSettings::LOOP },
            ));
        }
    }
}

fn play_sfx_events(
    mut commands: Commands,
    mut events: EventReader<PlaySfx>,
    handles: Res<AudioHandles>,
) {
    for ev in events.read() {
        let src = match ev.0 {
            SfxKind::Work => handles.work.clone(),
            SfxKind::Eat => handles.eat.clone(),
            SfxKind::Sleep => handles.sleep.clone(),
            SfxKind::Interact => handles.interact.clone(),
            SfxKind::Chime => handles.chime.clone(),
        };
        if let Some(src) = src {
            commands.spawn((AudioPlayer::<AudioSource>(src), PlaybackSettings::DESPAWN));
        }
    }
}

/// Crossfades weather ambient audio based on current WeatherKind.
/// Rain plays during Rainy, storm during Stormy, neither otherwise.
/// Thunder SFX fires on lightning flash onset.
fn update_weather_audio(
    mut commands: Commands,
    handles: Res<AudioHandles>,
    weather: Res<WeatherKind>,
    lightning: Res<LightningTimer>,
    weather_q: Query<Entity, With<WeatherAmbient>>,
) {
    let want_rain = *weather == WeatherKind::Rainy;
    let want_storm = weather.is_stormy();
    let want_any = want_rain || want_storm;

    if !want_any {
        // Stop weather audio when not raining/storming
        for e in &weather_q {
            commands.entity(e).despawn();
        }
        return;
    }

    // Already playing? keep it (audio swaps on weather change, which is daily)
    if !weather_q.is_empty() {
        // Fire thunder SFX on lightning flash onset
        if want_storm && lightning.flash_alpha > 0.30
            && let Some(src) = handles.thunder.clone()
        {
            commands.spawn((AudioPlayer::<AudioSource>(src), PlaybackSettings::DESPAWN));
        }
        return;
    }

    // Start the appropriate loop
    let (src, vol) = if want_storm {
        (handles.storm_loop.clone(), 0.30)
    } else {
        (handles.rain_loop.clone(), 0.22)
    };
    if let Some(src) = src {
        commands.spawn((
            WeatherAmbient,
            AudioPlayer::<AudioSource>(src),
            PlaybackSettings {
                volume: Volume::new(vol),
                ..PlaybackSettings::LOOP
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_disk_path_points_into_assets_dir() {
        let p = asset_disk_path("audio/ambient.ogg");
        assert!(p.to_string_lossy().contains("assets"));
        assert!(p.to_string_lossy().contains("ambient.ogg"));
    }

    #[test]
    fn shipped_audio_assets_exist() {
        // Core SFX assets must exist; BGM tracks are optional (CC downloads)
        assert!(asset_disk_path("audio/ambient.ogg").exists(), "ambient.ogg missing");
        assert!(asset_disk_path("audio/chime.ogg").exists(), "chime.ogg missing");
        assert!(asset_disk_path("audio/work.ogg").exists(), "work.ogg missing");
        assert!(asset_disk_path("audio/eat.ogg").exists(), "eat.ogg missing");
        assert!(asset_disk_path("audio/sleep.ogg").exists(), "sleep.ogg missing");
        assert!(asset_disk_path("audio/interact.ogg").exists(), "interact.ogg missing");
        // BGM tracks are optional; no assertion
    }
}
