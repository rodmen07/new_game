#![allow(clippy::type_complexity)]

use crate::components::*;
use crate::resources::*;
use bevy::prelude::*;

// ── Day / night cycle ─────────────────────────────────────────────────────────

/// Drives the world-space DayNightOverlay sprite color and alpha based on time,
/// season, and weather.
pub fn update_day_night(
    gt: Res<GameTime>,
    season: Res<Season>,
    weather: Res<WeatherKind>,
    mut overlay_q: Query<&mut Sprite, With<DayNightOverlay>>,
) {
    let Some(mut sprite) = overlay_q.iter_mut().next() else {
        return;
    };
    let h = gt.hours % 24.;

    let (r, g, b, a) = if (5. ..8.).contains(&h) {
        // Dawn — warm orange fading as day breaks
        let t = (h - 5.) / 3.;
        (
            0.55 * (1. - t),
            0.20 * (1. - t),
            0.08 * (1. - t),
            0.40 * (1. - t),
        )
    } else if (8. ..18.).contains(&h) {
        // Day — clear, only bad weather adds tint
        match *weather {
            WeatherKind::Stormy => (0.08, 0.07, 0.14, 0.16),
            WeatherKind::Rainy => (0.03, 0.05, 0.10, 0.08),
            _ => (0., 0., 0., 0.),
        }
    } else if (18. ..21.).contains(&h) {
        // Dusk — golden orange builds in
        let t = (h - 18.) / 3.;
        (0.55 * t, 0.20 * t, 0.03 * t, 0.24 * t)
    } else {
        // Night (21-5) — deep blue
        let t = if h >= 21. {
            ((h - 21.) / 3.).min(1.)
        } else {
            (1. - h / 5.).clamp(0., 1.)
        };
        (0.02, 0.04, 0.22, 0.50 * t)
    };

    let (sr, sg, sb, sa) = match season.current {
        SeasonKind::Spring => (0.00, 0.02, 0.00, 0.00),
        SeasonKind::Summer => (0.03, 0.00, -0.02, 0.00),
        SeasonKind::Autumn => (0.05, -0.01, -0.02, 0.00),
        SeasonKind::Winter => (-0.01, 0.01, 0.07, 0.04),
    };

    sprite.color = Color::srgba(
        (r + sr).clamp(0., 1.),
        (g + sg).clamp(0., 1.),
        (b + sb).clamp(0., 1.),
        (a + sa).clamp(0., 0.65),
    );
}

// ── Interactable proximity highlight ─────────────────────────────────────────

/// Pulses a yellow highlight box around whichever interactable the player is nearest.
pub fn update_highlight(
    nearby: Res<NearbyInteractable>,
    time: Res<Time>,
    interactable_q: Query<
        (&Transform, &ObjectSize),
        (With<Interactable>, Without<InteractHighlight>),
    >,
    mut highlight_q: Query<(&mut Transform, &mut Sprite), With<InteractHighlight>>,
) {
    let Some((mut htf, mut hsprite)) = highlight_q.iter_mut().next() else {
        return;
    };

    if let Some(entity) = nearby.entity
        && let Ok((tf, size)) = interactable_q.get(entity)
    {
        let pulse = (time.elapsed_secs() * 4.5).sin() * 0.5 + 0.5;
        let expand = 4. + pulse * 8.;
        let alpha = 0.12 + pulse * 0.26;

        htf.translation = Vec3::new(tf.translation.x, tf.translation.y, 1.98);
        hsprite.custom_size = Some(size.0 + Vec2::splat(expand));
        hsprite.color = Color::srgba(1., 1., 0.35, alpha);
        return;
    }
    hsprite.color = Color::srgba(1., 1., 0.5, 0.);
}

// ── Sprint trail particles ─────────────────────────────────────────────────────

#[derive(Component)]
pub struct Particle {
    pub lifetime: f32,
    pub max_lifetime: f32,
}

/// Spawns faint white dust particles behind the player while sprinting.
pub fn spawn_sprint_particles(
    mut commands: Commands,
    player_q: Query<(&Transform, &PlayerMovement), With<LocalPlayer>>,
    time: Res<Time>,
) {
    let Some((ptf, pm)) = player_q.iter().next() else {
        return;
    };
    if !pm.sprinting {
        return;
    };
    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    let count = ((dt * 80.).round() as u32).min(3);
    let dir = if pm.velocity.length() > 1. {
        pm.velocity.normalize()
    } else {
        Vec2::Y
    };
    for i in 0..count {
        let spread_angle = t * 8. + i as f32 * std::f32::consts::FRAC_PI_3;
        let spread = Vec2::new(spread_angle.cos(), spread_angle.sin()) * 3.;
        let back = -dir * (4. + i as f32 * 3.);
        let pos = Vec3::new(
            ptf.translation.x + back.x + spread.x,
            ptf.translation.y + back.y + spread.y,
            ptf.translation.z - 1.,
        );
        commands.spawn((
            Sprite {
                color: Color::srgba(0.92, 0.88, 0.80, 0.40),
                custom_size: Some(Vec2::splat(4.)),
                ..default()
            },
            Transform::from_translation(pos),
            Particle {
                lifetime: 0.14,
                max_lifetime: 0.14,
            },
        ));
    }
}

/// Ages particles and despawns them when expired.
pub fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particle_q: Query<(Entity, &mut Particle, &mut Sprite), Without<WeatherDrop>>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut sprite) in &mut particle_q {
        particle.lifetime -= dt;
        if particle.lifetime <= 0. {
            commands.entity(entity).despawn();
        } else {
            let t = (particle.lifetime / particle.max_lifetime).clamp(0., 1.);
            sprite.color = Color::srgba(0.92, 0.88, 0.80, t * 0.40);
        }
    }
}

// ── Weather particles ─────────────────────────────────────────────────────────

/// Spawns weather-appropriate particles around the camera viewport.
pub fn spawn_weather_particles(
    mut commands: Commands,
    weather: Res<WeatherKind>,
    season: Res<Season>,
    cam_q: Query<(&Transform, &OrthographicProjection), With<MainCamera>>,
    time: Res<Time>,
    _gt: Res<GameTime>,
) {
    let Some((cam_tf, proj)) = cam_q.iter().next() else {
        return;
    };
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let cx = cam_tf.translation.x;
    let cy = cam_tf.translation.y;
    let half_w = 960. * proj.scale;
    let half_h = 540. * proj.scale;
    let is_winter = season.current == SeasonKind::Winter;

    // Pseudo-random scatter using elapsed time
    let hash = |seed: u32| -> f32 {
        let v = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (v & 0xFFFF) as f32 / 65535.0
    };
    let frame_seed = (t * 1000.0) as u32;

    match *weather {
        WeatherKind::Rainy => {
            if is_winter {
                // Snow
                let count = ((dt * 240.).round() as u32).min(4);
                for i in 0..count {
                    let s = frame_seed.wrapping_add(i * 7919);
                    let x = cx + (hash(s) - 0.5) * half_w * 2.0;
                    let y = cy + half_h + 20.0 + hash(s.wrapping_add(1)) * 40.0;
                    let vx = (hash(s.wrapping_add(2)) - 0.5) * 20.0;
                    let vy = -40.0 - hash(s.wrapping_add(3)) * 20.0;
                    commands.spawn((
                        Sprite {
                            color: Color::srgba(0.95, 0.95, 1.0, 0.45),
                            custom_size: Some(Vec2::splat(3.0)),
                            ..default()
                        },
                        Transform::from_xyz(x, y, 48.0),
                        WeatherDrop {
                            vel: Vec2::new(vx, vy),
                            lifetime: 3.0,
                            max_lifetime: 3.0,
                            base_color: [0.95, 0.95, 1.0, 0.45],
                        },
                    ));
                }
            } else {
                // Rain
                let count = ((dt * 420.).round() as u32).min(7);
                for i in 0..count {
                    let s = frame_seed.wrapping_add(i * 6271);
                    let x = cx + (hash(s) - 0.5) * half_w * 2.0;
                    let y = cy + half_h + 10.0;
                    let vx = -20.0 - hash(s.wrapping_add(1)) * 20.0;
                    let vy = -260.0 - hash(s.wrapping_add(2)) * 90.0;
                    commands.spawn((
                        Sprite {
                            color: Color::srgba(0.6, 0.7, 0.9, 0.45),
                            custom_size: Some(Vec2::new(1.5, 5.0)),
                            ..default()
                        },
                        Transform::from_xyz(x, y, 48.0),
                        WeatherDrop {
                            vel: Vec2::new(vx, vy),
                            lifetime: 1.0,
                            max_lifetime: 1.0,
                            base_color: [0.6, 0.7, 0.9, 0.45],
                        },
                    ));
                }
            }
        }
        WeatherKind::Stormy => {
            if is_winter {
                // Blizzard
                let count = ((dt * 540.).round() as u32).min(9);
                for i in 0..count {
                    let s = frame_seed.wrapping_add(i * 4561);
                    let x = cx + (hash(s) - 0.5) * half_w * 2.2;
                    let y = cy + half_h + 30.0 + hash(s.wrapping_add(1)) * 60.0;
                    let vx = -30.0 - hash(s.wrapping_add(2)) * 40.0;
                    let vy = -60.0 - hash(s.wrapping_add(3)) * 40.0;
                    commands.spawn((
                        Sprite {
                            color: Color::srgba(0.90, 0.90, 1.0, 0.5),
                            custom_size: Some(Vec2::splat(4.0)),
                            ..default()
                        },
                        Transform::from_xyz(x, y, 48.0),
                        WeatherDrop {
                            vel: Vec2::new(vx, vy),
                            lifetime: 2.0,
                            max_lifetime: 2.0,
                            base_color: [0.90, 0.90, 1.0, 0.5],
                        },
                    ));
                }
            } else {
                // Heavy rain
                let count = ((dt * 660.).round() as u32).min(11);
                for i in 0..count {
                    let s = frame_seed.wrapping_add(i * 5347);
                    let x = cx + (hash(s) - 0.5) * half_w * 2.2;
                    let y = cy + half_h + 10.0;
                    let vx = -40.0 - hash(s.wrapping_add(1)) * 40.0;
                    let vy = -350.0 - hash(s.wrapping_add(2)) * 100.0;
                    commands.spawn((
                        Sprite {
                            color: Color::srgba(0.4, 0.5, 0.75, 0.55),
                            custom_size: Some(Vec2::new(1.5, 7.0)),
                            ..default()
                        },
                        Transform::from_xyz(x, y, 48.0),
                        WeatherDrop {
                            vel: Vec2::new(vx, vy),
                            lifetime: 0.8,
                            max_lifetime: 0.8,
                            base_color: [0.4, 0.5, 0.75, 0.55],
                        },
                    ));
                }
            }
        }
        WeatherKind::Cloudy => {
            // Autumn leaves or Spring petals on cloudy days
            match season.current {
                SeasonKind::Autumn if hash(frame_seed) < dt * 60.0 => {
                    let s = frame_seed.wrapping_add(3301);
                    let x = cx + (hash(s) - 0.5) * half_w * 2.0;
                    let y = cy + half_h + 20.0;
                    let vx = 15.0 + hash(s.wrapping_add(1)) * 15.0;
                    let vy = -20.0 - hash(s.wrapping_add(2)) * 20.0;
                    // Varied leaf colors
                    let color_pick = (s.wrapping_mul(7) % 3) as usize;
                    let colors = [
                        [0.80, 0.45, 0.15, 0.50], // orange
                        [0.70, 0.25, 0.10, 0.50], // red-brown
                        [0.85, 0.65, 0.15, 0.50], // golden
                    ];
                    let c = colors[color_pick];
                    commands.spawn((
                        Sprite {
                            color: Color::srgba(c[0], c[1], c[2], c[3]),
                            custom_size: Some(Vec2::new(4.0, 3.0)),
                            ..default()
                        },
                        Transform::from_xyz(x, y, 48.0),
                        WeatherDrop {
                            vel: Vec2::new(vx, vy),
                            lifetime: 2.5,
                            max_lifetime: 2.5,
                            base_color: c,
                        },
                    ));
                }
                SeasonKind::Spring if hash(frame_seed.wrapping_add(99)) < dt * 40.0 => {
                    let s = frame_seed.wrapping_add(2203);
                    let x = cx + (hash(s) - 0.5) * half_w * 2.0;
                    let y = cy + half_h + 20.0;
                    let vx = 5.0 + hash(s.wrapping_add(1)) * 10.0;
                    let vy = -15.0 - hash(s.wrapping_add(2)) * 15.0;
                    let pink = if s.is_multiple_of(2) {
                        [1.0, 0.75, 0.85, 0.40]
                    } else {
                        [1.0, 0.90, 0.95, 0.35]
                    };
                    commands.spawn((
                        Sprite {
                            color: Color::srgba(pink[0], pink[1], pink[2], pink[3]),
                            custom_size: Some(Vec2::splat(3.0)),
                            ..default()
                        },
                        Transform::from_xyz(x, y, 48.0),
                        WeatherDrop {
                            vel: Vec2::new(vx, vy),
                            lifetime: 2.5,
                            max_lifetime: 2.5,
                            base_color: pink,
                        },
                    ));
                }
                _ => {}
            }
        }
        WeatherKind::Sunny => {
            // Golden sparkles floating upward
            if hash(frame_seed.wrapping_add(777)) < dt * 20.0 {
                let s = frame_seed.wrapping_add(1117);
                let x = cx + (hash(s) - 0.5) * half_w * 1.6;
                let y = cy + (hash(s.wrapping_add(1)) - 0.5) * half_h * 1.6;
                let vx = (hash(s.wrapping_add(2)) - 0.5) * 6.0;
                let vy = 10.0 + hash(s.wrapping_add(3)) * 12.0;
                commands.spawn((
                    Sprite {
                        color: Color::srgba(1.0, 0.95, 0.5, 0.30),
                        custom_size: Some(Vec2::splat(2.0)),
                        ..default()
                    },
                    Transform::from_xyz(x, y, 48.0),
                    WeatherDrop {
                        vel: Vec2::new(vx, vy),
                        lifetime: 1.5,
                        max_lifetime: 1.5,
                        base_color: [1.0, 0.95, 0.5, 0.30],
                    },
                ));
            }
        }
    }

    // Splash particles when rain/storm drops hit ground level
    if matches!(*weather, WeatherKind::Rainy | WeatherKind::Stormy)
        && !is_winter
        && hash(frame_seed.wrapping_add(5555)) < dt * 180.0
    {
        let count = if weather.is_stormy() { 2 } else { 1 };
        for i in 0..count {
            let s = frame_seed.wrapping_add(8831 + i * 331);
            let x = cx + (hash(s) - 0.5) * half_w * 1.8;
            // Splash near ground or road level
            let y = cy + (hash(s.wrapping_add(1)) - 0.5) * half_h * 0.8 - half_h * 0.3;
            let vx = (hash(s.wrapping_add(2)) - 0.5) * 30.0;
            let vy = 10.0 + hash(s.wrapping_add(3)) * 15.0;
            commands.spawn((
                Sprite {
                    color: Color::srgba(0.7, 0.8, 0.95, 0.35),
                    custom_size: Some(Vec2::new(2.5, 1.5)),
                    ..default()
                },
                Transform::from_xyz(x, y, 2.0),
                WeatherDrop {
                    vel: Vec2::new(vx, vy),
                    lifetime: 0.2,
                    max_lifetime: 0.2,
                    base_color: [0.7, 0.8, 0.95, 0.35],
                },
            ));
        }
    }

    // Puddle shimmer during/after rain (small ground-level highlights)
    if matches!(*weather, WeatherKind::Rainy | WeatherKind::Stormy)
        && hash(frame_seed.wrapping_add(9999)) < dt * 30.0
    {
        let s = frame_seed.wrapping_add(4421);
        let x = cx + (hash(s) - 0.5) * half_w * 1.6;
        let y = (hash(s.wrapping_add(1)) - 0.5) * 20.0; // near road y=0
        let shimmer = (t * 3.0 + x * 0.01).sin() * 0.5 + 0.5;
        commands.spawn((
            Sprite {
                color: Color::srgba(0.5, 0.6, 0.8, 0.08 + shimmer * 0.06),
                custom_size: Some(Vec2::new(8.0 + shimmer * 4.0, 2.0)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.5),
            WeatherDrop {
                vel: Vec2::ZERO,
                lifetime: 0.6,
                max_lifetime: 0.6,
                base_color: [0.5, 0.6, 0.8, 0.12],
            },
        ));
    }
}

/// Moves weather particles by velocity and fades/despawns them.
pub fn update_weather_drops(
    mut commands: Commands,
    time: Res<Time>,
    mut drop_q: Query<(Entity, &mut WeatherDrop, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut drop, mut tf, mut sprite) in &mut drop_q {
        drop.lifetime -= dt;
        if drop.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        tf.translation.x += drop.vel.x * dt;
        tf.translation.y += drop.vel.y * dt;
        let fade = (drop.lifetime / drop.max_lifetime).clamp(0.0, 1.0);
        let c = &drop.base_color;
        sprite.color = Color::srgba(c[0], c[1], c[2], c[3] * fade);
    }
}

// ── Lightning flash ───────────────────────────────────────────────────────────

/// Triggers brief white flashes during storms and integrates with the day/night overlay.
pub fn update_lightning(
    weather: Res<WeatherKind>,
    mut lightning: ResMut<LightningTimer>,
    time: Res<Time>,
    mut overlay_q: Query<&mut Sprite, With<DayNightOverlay>>,
) {
    let dt = time.delta_secs();

    if !weather.is_stormy() {
        lightning.flash_alpha = 0.0;
        lightning.next_flash = 8.0;
        return;
    }

    // Decay flash
    if lightning.flash_alpha > 0.0 {
        lightning.flash_alpha = (lightning.flash_alpha - dt * 6.0).max(0.0);
    }

    // Countdown to next flash
    lightning.next_flash -= dt;
    if lightning.next_flash <= 0.0 {
        lightning.flash_alpha = 0.35;
        // Next flash in 6-14 seconds
        let t = time.elapsed_secs();
        let pseudo = ((t * 1000.0) as u32)
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        lightning.next_flash = 6.0 + (pseudo % 8000) as f32 / 1000.0;
    }

    // Apply flash to overlay
    if lightning.flash_alpha > 0.01 {
        let Some(mut sprite) = overlay_q.iter_mut().next() else {
            return;
        };
        let current = sprite.color.to_srgba();
        sprite.color = Color::srgba(
            (current.red + lightning.flash_alpha * 0.8).min(1.0),
            (current.green + lightning.flash_alpha * 0.8).min(1.0),
            (current.blue + lightning.flash_alpha * 0.7).min(1.0),
            (current.alpha + lightning.flash_alpha * 0.3).min(0.8),
        );
    }
}

// ── Y-sort ────────────────────────────────────────────────────────────────────

/// Top-down 2D depth sorting: each frame, recompute the z component of every
/// `YSort`-tagged entity so entities further south render in front. Uses a
/// small per-pixel slope (Y_SORT_SLOPE) around `base_z` so all characters
/// stay in the same depth band but layer correctly relative to each other.
///
/// Conventions:
/// - Higher world `y` (further north) -> lower z (drawn behind).
/// - Lower world `y` (further south)  -> higher z (drawn in front).
/// - Width of the sort band stays inside +/- (Y_SORT_RANGE * Y_SORT_SLOPE)
///   so y-sorted things never punch through tilemap (z=0) or the day/night
///   overlay (z=50).
const Y_SORT_SLOPE: f32 = 0.001;
const Y_SORT_RANGE: f32 = 4000.0;

pub fn apply_y_sort(mut q: Query<(&YSort, &mut Transform)>) {
    for (ys, mut tf) in &mut q {
        let clamped_y = tf.translation.y.clamp(-Y_SORT_RANGE, Y_SORT_RANGE);
        tf.translation.z = ys.base_z - clamped_y * Y_SORT_SLOPE;
    }
}

// ── Facing + walk-cycle scaffolding ──────────────────────────────────────────

/// Velocity threshold (world units / sec) below which an entity is considered
/// idle. Below this, `Facing` is preserved (so the character keeps looking
/// the way they were going) and `AnimFrame.idx` resets to 0.
const FACING_MOVE_EPSILON: f32 = 8.0;
/// Seconds per walk-cycle frame (4 frames per cycle).
const ANIM_FRAME_DURATION: f32 = 0.15;

/// Update `Facing` from `PlayerMovement.velocity` for the local player.
/// East/West dominates only when |vx| > |vy|, so diagonal-up still reads as
/// "up" - matches Stardew/Pokemon conventions.
pub fn update_player_facing(mut q: Query<(&PlayerMovement, &mut Facing), With<LocalPlayer>>) {
    for (pm, mut facing) in &mut q {
        let v = pm.velocity;
        if v.length_squared() < FACING_MOVE_EPSILON * FACING_MOVE_EPSILON {
            continue;
        }
        let new_facing = if v.x.abs() > v.y.abs() {
            if v.x > 0.0 {
                Facing::East
            } else {
                Facing::West
            }
        } else if v.y > 0.0 {
            Facing::North
        } else {
            Facing::South
        };
        if *facing != new_facing {
            *facing = new_facing;
        }
    }
}

/// Update `Facing` from `Npc.velocity`. Same rules as the player.
pub fn update_npc_facing(mut q: Query<(&Npc, &mut Facing)>) {
    for (npc, mut facing) in &mut q {
        let v = npc.velocity;
        if v.length_squared() < FACING_MOVE_EPSILON * FACING_MOVE_EPSILON {
            continue;
        }
        let new_facing = if v.x.abs() > v.y.abs() {
            if v.x > 0.0 {
                Facing::East
            } else {
                Facing::West
            }
        } else if v.y > 0.0 {
            Facing::North
        } else {
            Facing::South
        };
        if *facing != new_facing {
            *facing = new_facing;
        }
    }
}

/// Advance 4-frame walk cycles. Idle entities snap back to frame 0.
/// Today this drives no visuals (procedural humans have no spritesheet),
/// but the data is wired so dropping in pixel-art sheets later is mechanical.
#[allow(clippy::type_complexity)]
pub fn update_anim_frames(
    time: Res<Time>,
    mut q: Query<(&mut AnimFrame, Option<&PlayerMovement>, Option<&Npc>)>,
) {
    let dt = time.delta_secs();
    for (mut anim, pm, npc) in &mut q {
        let speed_sq = pm
            .map(|p| p.velocity.length_squared())
            .or_else(|| npc.map(|n| n.velocity.length_squared()))
            .unwrap_or(0.0);
        if speed_sq < FACING_MOVE_EPSILON * FACING_MOVE_EPSILON {
            anim.idx = 0;
            anim.timer = 0.0;
            continue;
        }
        anim.timer += dt;
        while anim.timer >= ANIM_FRAME_DURATION {
            anim.timer -= ANIM_FRAME_DURATION;
            anim.idx = (anim.idx + 1) % 4;
        }
    }
}

// ── Streetlamp glow ──────────────────────────────────────────────────────────

/// Marks an additive glow sprite that brightens at night. The base alpha is
/// driven by hour-of-day in `update_streetlamp_glow`.
#[derive(Component)]
pub struct StreetlampGlow;

/// Update the alpha of all `StreetlampGlow` sprites based on the current hour.
/// Glows are bright (alpha ~0.55) between 19:00 and 06:00, off in daylight.
pub fn update_streetlamp_glow(gt: Res<GameTime>, mut q: Query<&mut Sprite, With<StreetlampGlow>>) {
    let h = gt.hours % 24.0;
    let target_alpha = if !(6.0..19.0).contains(&h) {
        0.55
    } else if (18.0..19.0).contains(&h) {
        // 18:00-19:00 ramp up
        (h - 18.0) * 0.55
    } else if (5.0..6.0).contains(&h) {
        // 05:00-06:00 ramp down
        (6.0 - h) * 0.55
    } else {
        0.0
    };
    for mut sprite in &mut q {
        let mut c = sprite.color.to_srgba();
        c.alpha = target_alpha;
        sprite.color = Color::Srgba(c);
    }
}

// ── Neon shop signs ──────────────────────────────────────────────────────────

/// Marks a glowing sign sprite for a commercial zone. The hue is baked into
/// `base_color`; the alpha is animated by `update_neon_signs` so the sign
/// "switches on" at dusk with a subtle flicker.
#[derive(Component, Clone, Copy)]
pub struct NeonSign {
    pub base_color: Color,
}

/// Pulse neon-sign alpha between dusk and dawn with a small flicker.
/// Off (alpha 0) during daylight; ramps up 18:00-19:00, down 05:00-06:00.
pub fn update_neon_signs(
    gt: Res<GameTime>,
    time: Res<Time>,
    mut q: Query<(&NeonSign, &mut Sprite)>,
) {
    let h = gt.hours % 24.0;
    let base = if !(6.0..19.0).contains(&h) {
        0.85
    } else if (18.0..19.0).contains(&h) {
        (h - 18.0) * 0.85
    } else if (5.0..6.0).contains(&h) {
        (6.0 - h) * 0.85
    } else {
        0.0
    };
    // Subtle flicker (sin) so the signs feel alive.
    let t = time.elapsed_secs();
    let flicker = if base > 0.0 {
        1.0 + 0.06 * (t * 4.7).sin() + 0.03 * (t * 11.3).sin()
    } else {
        1.0
    };
    let alpha = (base * flicker).clamp(0.0, 1.0);
    for (sign, mut sprite) in &mut q {
        let mut c = sign.base_color.to_srgba();
        c.alpha = alpha;
        sprite.color = Color::Srgba(c);
    }
}

// ── Player pixel-art sheet hooks ─────────────────────────────────────────────

/// Width / height of a single sprite cell on the player sheet (pixels).
/// Stardew Valley uses 16x32; we pick 32x32 for now to match the procedural
/// player's relative footprint. Tune when real art is dropped in.
pub const PLAYER_SHEET_CELL: u32 = 32;
/// Number of walk-cycle frames per row. Row order: South, North, East, West.
pub const PLAYER_SHEET_FRAMES: u32 = 4;
/// Number of facing rows on the sheet.
pub const PLAYER_SHEET_ROWS: u32 = 4;

/// Maps a `Facing` enum to the row index on `art/characters/player.png`.
/// Row layout: 0 = South (default), 1 = North, 2 = East, 3 = West.
pub fn facing_row(f: Facing) -> u32 {
    match f {
        Facing::South => 0,
        Facing::North => 1,
        Facing::East => 2,
        Facing::West => 3,
    }
}

/// Computes the atlas index for a given facing + walk frame.
pub fn player_atlas_index(facing: Facing, frame: u8) -> usize {
    (facing_row(facing) * PLAYER_SHEET_FRAMES + frame as u32 % PLAYER_SHEET_FRAMES) as usize
}

/// Startup system: kicks off async load of `art/characters/player.png` and
/// builds the matching `TextureAtlasLayout`. Both handles live in `ArtAssets`.
/// Missing-asset is non-fatal: Bevy logs a warning and the procedural body
/// stays visible.
pub fn init_art_assets(
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut art: ResMut<ArtAssets>,
) {
    art.player_sheet = Some(asset_server.load("art/characters/player.png"));
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(PLAYER_SHEET_CELL),
        PLAYER_SHEET_FRAMES,
        PLAYER_SHEET_ROWS,
        None,
        None,
    );
    art.player_atlas = Some(layouts.add(layout));
}

/// When `art/characters/player.png` finishes loading, swap the procedural
/// body off and reveal the sprite-sheet child. Until then (or if the file
/// is missing), the procedural body stays visible and this system is a
/// no-op aside from updating the atlas index.
pub fn update_player_sheet(
    art: Res<ArtAssets>,
    asset_server: Res<AssetServer>,
    mut sheet_q: Query<
        (&mut Sprite, &mut Visibility),
        (With<PlayerSheetSprite>, Without<ProceduralBody>),
    >,
    mut body_q: Query<&mut Visibility, (With<ProceduralBody>, Without<PlayerSheetSprite>)>,
    player_q: Query<(&Facing, &AnimFrame), With<LocalPlayer>>,
) {
    let Some(sheet_handle) = art.player_sheet.as_ref() else {
        return;
    };
    let Some(layout_handle) = art.player_atlas.as_ref() else {
        return;
    };
    let loaded = matches!(
        asset_server.get_load_state(sheet_handle),
        Some(bevy::asset::LoadState::Loaded)
    );
    let Ok((facing, anim)) = player_q.get_single() else {
        return;
    };
    let index = player_atlas_index(*facing, anim.idx);
    for (mut sprite, mut vis) in &mut sheet_q {
        if loaded {
            sprite.image = sheet_handle.clone();
            sprite.texture_atlas = Some(TextureAtlas {
                layout: layout_handle.clone(),
                index,
            });
            *vis = Visibility::Inherited;
        } else {
            *vis = Visibility::Hidden;
        }
    }
    for mut vis in &mut body_q {
        *vis = if loaded {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
