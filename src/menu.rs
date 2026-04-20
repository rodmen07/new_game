#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use crate::resources::Notification;
use crate::save::{PendingLoad, SaveRequest, load_save_data};
use crate::settings::{Difficulty, GameSettings};
use bevy::prelude::*;

const BTN_NORMAL: Color = Color::srgb(0.15, 0.15, 0.18);
const BTN_HOVERED: Color = Color::srgb(0.28, 0.28, 0.36);
const BTN_PRESSED: Color = Color::srgb(0.10, 0.38, 0.58);
const BTN_DANGER: Color = Color::srgb(0.40, 0.08, 0.08);

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    Playing,
    Paused,
    Settings,
}

// ── Tracks why we're entering Playing — lets OnEnter decide reset vs resume ──

#[derive(Resource, Default, PartialEq, Clone, Copy)]
pub enum GameStartKind {
    NewGame,
    Continue,
    #[default]
    Resume,
}

pub fn reset_start_kind(mut sk: ResMut<GameStartKind>) {
    *sk = GameStartKind::Resume;
}

// ── Marker components ─────────────────────────────────────────────────────────

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct PauseRoot;

#[derive(Component)]
struct SettingsRoot;

/// Marks the text entity that displays the current volume level.
#[derive(Component)]
struct VolumeDisplay;

#[derive(Component)]
enum MenuButton {
    NewGame,
    Continue,
    Resume,
    MainMenu,
    Settings,
    BackFromSettings,
    Quit,
    SetDifficulty(Difficulty),
    VolumeDown,
    VolumeUp,
}

/// Stores the button's resting background color so hover/press can restore it.
#[derive(Component, Clone, Copy)]
struct BaseColor(Color);

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .init_resource::<GameStartKind>()
            .add_systems(OnEnter(AppState::Menu), spawn_main_menu)
            .add_systems(OnExit(AppState::Menu), despawn::<MenuRoot>)
            .add_systems(OnEnter(AppState::Paused), spawn_pause_menu)
            .add_systems(OnExit(AppState::Paused), despawn::<PauseRoot>)
            .add_systems(OnEnter(AppState::Settings), spawn_settings_menu)
            .add_systems(OnExit(AppState::Settings), despawn::<SettingsRoot>)
            .add_systems(
                Update,
                (update_settings_highlight, update_volume_display)
                    .run_if(in_state(AppState::Settings)),
            )
            .add_systems(Update, (handle_menu_buttons, handle_escape));
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn despawn<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

fn spawn_btn(parent: &mut ChildBuilder, label: &str, kind: MenuButton, color: Color) {
    parent
        .spawn((
            Button,
            kind,
            BaseColor(color),
            Node {
                width: Val::Px(260.),
                height: Val::Px(52.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.)),
                ..default()
            },
            BackgroundColor(color),
            BorderColor(Color::srgb(0.35, 0.35, 0.45)),
            BorderRadius::all(Val::Px(4.)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(label),
                TextFont {
                    font_size: 20.,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Smaller button used for difficulty selection and volume controls in the settings screen.
fn spawn_small_btn(parent: &mut ChildBuilder, label: &str, kind: MenuButton, color: Color) {
    parent
        .spawn((
            Button,
            kind,
            BaseColor(color),
            Node {
                width: Val::Px(120.),
                height: Val::Px(42.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.)),
                ..default()
            },
            BackgroundColor(color),
            BorderColor(Color::srgb(0.35, 0.35, 0.45)),
            BorderRadius::all(Val::Px(4.)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(label),
                TextFont {
                    font_size: 18.,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

// ── Main menu ─────────────────────────────────────────────────────────────────

fn spawn_main_menu(mut commands: Commands) {
    let has_save = crate::save::save_exists();

    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.97)),
            ZIndex(100),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("Everyday Life Simulator"),
                TextFont {
                    font_size: 52.,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            p.spawn((
                Text::new("Balance work, health, and happiness"),
                TextFont {
                    font_size: 18.,
                    ..default()
                },
                TextColor(Color::srgb(0.60, 0.60, 0.65)),
                Node {
                    margin: UiRect::bottom(Val::Px(28.)),
                    ..default()
                },
            ));
            if has_save {
                spawn_btn(p, "Continue", MenuButton::Continue, BTN_PRESSED);
            }
            spawn_btn(p, "New Game", MenuButton::NewGame, BTN_NORMAL);
        });
}

// ── Pause menu ────────────────────────────────────────────────────────────────

fn spawn_pause_menu(mut commands: Commands) {
    commands
        .spawn((
            PauseRoot,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.),
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.72)),
            ZIndex(100),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 42.,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(20.)),
                    ..default()
                },
            ));
            spawn_btn(p, "Resume", MenuButton::Resume, BTN_NORMAL);
            spawn_btn(p, "Settings", MenuButton::Settings, BTN_NORMAL);
            spawn_btn(p, "Main Menu", MenuButton::MainMenu, BTN_NORMAL);
            spawn_btn(p, "Quit", MenuButton::Quit, BTN_DANGER);
        });
}

// ── Button interaction ────────────────────────────────────────────────────────

fn handle_menu_buttons(
    mut q: Query<
        (&Interaction, &MenuButton, &mut BackgroundColor, &BaseColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut next: ResMut<NextState<AppState>>,
    mut start_kind: ResMut<GameStartKind>,
    mut pending: ResMut<PendingLoad>,
    mut save_ev: EventWriter<SaveRequest>,
    mut app_exit: EventWriter<AppExit>,
    mut settings: ResMut<GameSettings>,
    mut global_vol: ResMut<GlobalVolume>,
    mut notif: ResMut<Notification>,
) {
    for (interaction, button, mut bg, base) in &mut q {
        match interaction {
            Interaction::Hovered => {
                // Difficulty buttons skip hover so the selection highlight stays clean
                if !matches!(button, MenuButton::SetDifficulty(_)) {
                    *bg = BackgroundColor(BTN_HOVERED);
                }
            }
            Interaction::None => *bg = BackgroundColor(base.0),
            Interaction::Pressed => {
                *bg = BackgroundColor(BTN_PRESSED);
                match button {
                    MenuButton::NewGame => {
                        *start_kind = GameStartKind::NewGame;
                        next.set(AppState::Playing);
                    }
                    MenuButton::Continue => {
                        pending.0 = load_save_data();
                        *start_kind = GameStartKind::Continue;
                        next.set(AppState::Playing);
                    }
                    MenuButton::Resume => {
                        *start_kind = GameStartKind::Resume;
                        next.set(AppState::Playing);
                    }
                    MenuButton::MainMenu => {
                        save_ev.send_default();
                        next.set(AppState::Menu);
                    }
                    MenuButton::Settings => {
                        next.set(AppState::Settings);
                    }
                    MenuButton::BackFromSettings => {
                        next.set(AppState::Paused);
                    }
                    MenuButton::SetDifficulty(d) => {
                        settings.difficulty = *d;
                        if !settings.save() {
                            notif.push("Failed to save settings".to_string(), 3.);
                        }
                    }
                    MenuButton::VolumeDown => {
                        let v = (settings.master_volume - 0.1).max(0.);
                        settings.master_volume = (v * 10.).round() / 10.;
                        if !settings.save() {
                            notif.push("Failed to save settings".to_string(), 3.);
                        }
                        global_vol.volume = bevy::audio::Volume::new(settings.master_volume);
                    }
                    MenuButton::VolumeUp => {
                        let v = (settings.master_volume + 0.1).min(1.);
                        settings.master_volume = (v * 10.).round() / 10.;
                        if !settings.save() {
                            notif.push("Failed to save settings".to_string(), 3.);
                        }
                        global_vol.volume = bevy::audio::Volume::new(settings.master_volume);
                    }
                    MenuButton::Quit => {
                        app_exit.send(AppExit::Success);
                    }
                }
            }
        }
    }
}

// ── Escape key ────────────────────────────────────────────────────────────────

fn handle_escape(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut start_kind: ResMut<GameStartKind>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        match state.get() {
            AppState::Playing => next.set(AppState::Paused),
            AppState::Paused => {
                *start_kind = GameStartKind::Resume;
                next.set(AppState::Playing);
            }
            AppState::Settings => next.set(AppState::Paused),
            AppState::Menu => {}
        }
    }
}

// ── Settings screen ───────────────────────────────────────────────────────────

fn spawn_settings_menu(mut commands: Commands, settings: Res<GameSettings>) {
    commands
        .spawn((
            SettingsRoot,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.),
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.88)),
            ZIndex(100),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("Settings"),
                TextFont {
                    font_size: 40.,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(8.)),
                    ..default()
                },
            ));

            // Difficulty
            p.spawn((
                Text::new("Difficulty"),
                TextFont {
                    font_size: 16.,
                    ..default()
                },
                TextColor(Color::srgb(0.65, 0.65, 0.72)),
            ));
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.),
                ..default()
            })
            .with_children(|row| {
                for d in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
                    let color = if d == settings.difficulty {
                        BTN_PRESSED
                    } else {
                        BTN_NORMAL
                    };
                    spawn_small_btn(row, d.label(), MenuButton::SetDifficulty(d), color);
                }
            });

            // Volume
            p.spawn((
                Text::new("Volume"),
                TextFont {
                    font_size: 16.,
                    ..default()
                },
                TextColor(Color::srgb(0.65, 0.65, 0.72)),
                Node {
                    margin: UiRect::top(Val::Px(8.)),
                    ..default()
                },
            ));
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                spawn_small_btn(row, "  −  ", MenuButton::VolumeDown, BTN_NORMAL);
                row.spawn((
                    Text::new(format!("{:.1}", settings.master_volume)),
                    TextFont {
                        font_size: 22.,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    VolumeDisplay,
                    Node {
                        width: Val::Px(48.),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ));
                spawn_small_btn(row, "  +  ", MenuButton::VolumeUp, BTN_NORMAL);
            });

            spawn_btn(p, "Back", MenuButton::BackFromSettings, BTN_NORMAL);
        });
}

/// Keeps difficulty button highlight in sync with the active difficulty.
fn update_settings_highlight(
    settings: Res<GameSettings>,
    mut q: Query<(&MenuButton, &mut BaseColor, &mut BackgroundColor)>,
) {
    for (btn, mut base, mut bg) in &mut q {
        if let MenuButton::SetDifficulty(d) = btn {
            let color = if *d == settings.difficulty {
                BTN_PRESSED
            } else {
                BTN_NORMAL
            };
            base.0 = color;
            *bg = BackgroundColor(color);
        }
    }
}

/// Refreshes the volume display label every frame while in Settings.
fn update_volume_display(
    settings: Res<GameSettings>,
    mut q: Query<&mut Text, With<VolumeDisplay>>,
) {
    for mut text in &mut q {
        *text = Text::new(format!("{:.1}", settings.master_volume));
    }
}
