use bevy::prelude::*;
use crate::components::*;
use crate::resources::*;

// ── Day / night cycle ─────────────────────────────────────────────────────────

/// Drives the world-space DayNightOverlay sprite color and alpha based on time,
/// season, and weather.
pub fn update_day_night(
    gt:      Res<GameTime>,
    season:  Res<Season>,
    weather: Res<WeatherKind>,
    mut overlay_q: Query<&mut Sprite, With<DayNightOverlay>>,
) {
    let Ok(mut sprite) = overlay_q.get_single_mut() else { return };
    let h = gt.hours % 24.;

    let (r, g, b, a) = if h >= 5. && h < 8. {
        // Dawn — warm orange fading as day breaks
        let t = (h - 5.) / 3.;
        (0.55 * (1. - t), 0.20 * (1. - t), 0.08 * (1. - t), 0.40 * (1. - t))
    } else if h >= 8. && h < 18. {
        // Day — clear, only bad weather adds tint
        match *weather {
            WeatherKind::Stormy => (0.08, 0.07, 0.14, 0.16),
            WeatherKind::Rainy  => (0.03, 0.05, 0.10, 0.08),
            _ => (0., 0., 0., 0.),
        }
    } else if h >= 18. && h < 21. {
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
        SeasonKind::Spring => ( 0.00,  0.02,  0.00,  0.00),
        SeasonKind::Summer => ( 0.03,  0.00, -0.02,  0.00),
        SeasonKind::Autumn => ( 0.05, -0.01, -0.02,  0.00),
        SeasonKind::Winter => (-0.01,  0.01,  0.07,  0.04),
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
    nearby:          Res<NearbyInteractable>,
    time:            Res<Time>,
    interactable_q:  Query<(&Transform, &ObjectSize), (With<Interactable>, Without<InteractHighlight>)>,
    mut highlight_q: Query<(&mut Transform, &mut Sprite), With<InteractHighlight>>,
) {
    let Ok((mut htf, mut hsprite)) = highlight_q.get_single_mut() else { return };

    if let Some(entity) = nearby.entity {
        if let Ok((tf, size)) = interactable_q.get(entity) {
            let pulse  = (time.elapsed_secs() * 4.5).sin() * 0.5 + 0.5;
            let expand = 4. + pulse * 8.;
            let alpha  = 0.12 + pulse * 0.26;

            htf.translation = Vec3::new(tf.translation.x, tf.translation.y, 1.98);
            hsprite.custom_size = Some(size.0 + Vec2::splat(expand));
            hsprite.color = Color::srgba(1., 1., 0.35, alpha);
            return;
        }
    }
    hsprite.color = Color::srgba(1., 1., 0.5, 0.);
}

// ── Dash trail particles ──────────────────────────────────────────────────────

#[derive(Component)]
pub struct Particle {
    pub lifetime:     f32,
    pub max_lifetime: f32,
}

/// Spawns orange trail particles behind the player while dashing.
pub fn spawn_dash_particles(
    mut commands: Commands,
    pm:        Res<PlayerMovement>,
    player_q:  Query<&Transform, With<Player>>,
    time:      Res<Time>,
) {
    if !pm.dashing { return; }
    let Ok(ptf) = player_q.get_single() else { return };
    let t = time.elapsed_secs();

    for i in 0..4u32 {
        let perp_angle = t * 20. + i as f32 * std::f32::consts::FRAC_PI_2;
        let perp = Vec2::new(perp_angle.cos(), perp_angle.sin()) * 5.;
        let back = -pm.dash_dir * (5. + i as f32 * 5.);
        let pos  = Vec3::new(
            ptf.translation.x + back.x + perp.x,
            ptf.translation.y + back.y + perp.y,
            ptf.translation.z - 0.5,
        );
        let size  = 6. - i as f32;
        let alpha = 0.78 - i as f32 * 0.16;

        commands.spawn((
            Sprite {
                color: Color::srgba(1.0, 0.70, 0.25, alpha),
                custom_size: Some(Vec2::splat(size.max(1.))),
                ..default()
            },
            Transform::from_translation(pos),
            Particle { lifetime: 0.20, max_lifetime: 0.20 },
        ));
    }
}

/// Ages dash particles and removes them when expired.
pub fn update_particles(
    mut commands:   Commands,
    time:           Res<Time>,
    mut particle_q: Query<(Entity, &mut Particle, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut sprite) in &mut particle_q {
        particle.lifetime -= dt;
        if particle.lifetime <= 0. {
            commands.entity(entity).despawn();
        } else {
            let t = (particle.lifetime / particle.max_lifetime).clamp(0., 1.);
            sprite.color = Color::srgba(1.0, 0.70, 0.25, t * 0.78);
        }
    }
}

