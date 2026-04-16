use bevy::prelude::*;
use crate::resources::*;

pub fn decay_stats(
    mut stats: ResMut<PlayerStats>,
    time: Res<Time>,
    weather: Res<WeatherKind>,
) {
    let dt = time.delta_secs();
    let energy_mult = weather.energy_decay_mult();
    stats.hunger = (stats.hunger + dt * 0.80).min(100.);
    stats.energy = (stats.energy - dt * 0.55 * energy_mult).max(0.);
    if stats.meditation_buff > 0. { stats.meditation_buff = (stats.meditation_buff - dt).max(0.); }
    if stats.hunger > 60. { stats.happiness = (stats.happiness - dt * 0.25).max(0.); }
    if stats.energy < 20. { stats.happiness = (stats.happiness - dt * 0.25).max(0.); }
}

pub fn degrade_health(
    mut stats: ResMut<PlayerStats>,
    mut gs: ResMut<GameState>,
    conds: Res<Conditions>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let mal_mult = if conds.malnourished { 1.5 } else { 1.0 };
    if stats.hunger > 80. { stats.health = (stats.health - dt * 0.60 * mal_mult).max(0.); }
    if stats.energy < 10. { stats.health = (stats.health - dt * 0.40 * mal_mult).max(0.); }
    if stats.stress > 85. {
        stats.health = (stats.health - dt * 0.30).max(0.);
        gs.high_stress_today = true;
    }
    // Flag sustained high hunger (> 80) — used to trigger malnourishment after 3 days
    if stats.hunger > 80. { gs.high_hunger_today = true; }
}

pub fn check_critical(stats: Res<PlayerStats>, mut notif: ResMut<Notification>) {
    if notif.timer > 0. { return; }
    if stats.health < 15. {
        notif.message = "CRITICAL HEALTH! Eat and rest now!".into(); notif.timer = 4.; return;
    }
    if stats.hunger > 90. {
        notif.message = "Starving! Eat immediately!".into(); notif.timer = 3.; return;
    }
    if stats.energy < 5. {
        notif.message = "Exhausted! You need sleep.".into(); notif.timer = 3.; return;
    }
}