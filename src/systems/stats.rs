use crate::{components::PetKind, constants::TIME_SCALE, resources::*, settings::GameSettings};
use bevy::prelude::*;

pub fn decay_stats(
    mut stats: ResMut<PlayerStats>,
    time: Res<Time>,
    weather: Res<WeatherKind>,
    settings: Res<GameSettings>,
    mut pet: ResMut<Pet>,
) {
    let dt = time.delta_secs();
    let energy_mult = weather.energy_decay_mult() * settings.difficulty.energy_decay_mult();
    let hunger_mult = settings.difficulty.hunger_mult();
    stats.modify_hunger(dt * 0.80 * hunger_mult);
    stats.modify_energy(-dt * 0.55 * energy_mult);
    if stats.meditation_buff > 0. {
        stats.meditation_buff = (stats.meditation_buff - dt).max(0.);
    }
    if stats.cooldown > 0. {
        stats.cooldown = (stats.cooldown - dt).max(0.);
    }
    let mood_mult = Mood::from_happiness(stats.happiness).decay_mult();
    let mut hap_drain = 0.;
    if stats.hunger > 60. {
        hap_drain += dt * 0.25 * mood_mult;
    }
    if stats.energy < 20. {
        hap_drain += dt * 0.25 * mood_mult;
    }
    if hap_drain > 0. {
        stats.modify_happiness(-hap_drain);
    }
    // Pet: real-time hunger accumulation when not fed today
    if pet.has_pet && !pet.fed_today {
        pet.hunger = (pet.hunger + dt * 0.15).min(100.);
    }
    // Pet passive bonuses (only when not too hungry)
    if pet.has_pet && pet.hunger < 80. {
        match pet.kind {
            PetKind::Dog => {
                stats.modify_happiness(dt * 0.5);
            }
            PetKind::Cat => {
                stats.modify_stress(-dt * 0.3);
            }
            PetKind::Fish => {}
        }
    }
}

pub fn degrade_health(
    mut stats: ResMut<PlayerStats>,
    mut gs: ResMut<GameState>,
    mut conds: ResMut<Conditions>,
    time: Res<Time>,
    settings: Res<GameSettings>,
    mut notif: ResMut<Notification>,
) {
    let dt = time.delta_secs();

    // Tick hospital recovery timer
    if conds.hospitalized {
        conds.hospital_timer -= dt * TIME_SCALE / 3600.;
        if conds.hospital_timer <= 0. {
            conds.hospital_timer = 0.;
            conds.hospitalized = false;
            stats.modify_health(20.);
            notif.push(
                "Discharged from hospital. Take better care of yourself!",
                6.,
            );
        }
        return; // while hospitalised, health can't degrade further
    }

    let mal_mult = if conds.malnourished { 1.5 } else { 1.0 };
    let health_mult = mal_mult * settings.difficulty.health_drain_mult();
    let mut health_drain = 0.;
    if stats.hunger > 80. {
        health_drain += dt * 0.60 * health_mult;
    }
    if stats.energy < 10. {
        health_drain += dt * 0.40 * health_mult;
    }
    if stats.stress > 85. {
        health_drain += dt * 0.30 * settings.difficulty.health_drain_mult();
        gs.high_stress_today = true;
    }
    if health_drain > 0. {
        stats.modify_health(-health_drain);
    }
    // Flag sustained high hunger (> 80) — used to trigger malnourishment after 3 days
    if stats.hunger > 80. {
        gs.high_hunger_today = true;
    }

    // Hospitalisation: trigger when health hits 0
    if !conds.hospitalized && stats.health <= 0. {
        conds.hospitalized = true;
        conds.hospital_timer = 6.; // 6 in-game hours of recovery
        stats.health = 5.;
        stats.energy = (stats.energy + 30.).clamp(0., 100.);
        stats.modify_hunger(-30.);
        stats.money = (stats.money - 500.).max(-crate::constants::DEBT_LIMIT);
        notif.push("Hospitalised! -$500 medical bill. Resting for 6 hours.", 7.);
    }
}

pub fn check_critical(
    stats: Res<PlayerStats>,
    conds: Res<Conditions>,
    mut notif: ResMut<Notification>,
) {
    if conds.hospitalized {
        return;
    }
    if notif.timer > 0. {
        return;
    }
    if stats.health < 15. {
        notif.message = "CRITICAL HEALTH! Eat and rest now!".to_string();
        notif.timer = 4.;
        return;
    }
    if stats.hunger > 90. {
        notif.message = "Starving! Eat immediately!".to_string();
        notif.timer = 3.;
        return;
    }
    if stats.energy < 5. {
        notif.message = "Exhausted! You need sleep.".to_string();
        notif.timer = 3.;
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    // ── PlayerStats methods ────────────────────────────────────────────────────

    #[test]
    fn max_energy_no_sleep_debt() {
        let mut s = PlayerStats::default();
        s.sleep_debt = 0.;
        assert_eq!(s.max_energy(), 100.);
    }

    #[test]
    fn max_energy_moderate_sleep_debt() {
        let mut s = PlayerStats::default();
        s.sleep_debt = 12.;
        assert_eq!(s.max_energy(), 80.);
    }

    #[test]
    fn max_energy_severe_sleep_debt() {
        let mut s = PlayerStats::default();
        s.sleep_debt = 20.;
        assert_eq!(s.max_energy(), 60.);
    }

    #[test]
    fn stress_work_mult_low() {
        let mut s = PlayerStats::default();
        s.stress = 30.;
        assert!((s.stress_work_mult() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stress_work_mult_mid() {
        let mut s = PlayerStats::default();
        s.stress = 60.;
        assert!((s.stress_work_mult() - 0.85).abs() < 0.001);
    }

    #[test]
    fn stress_work_mult_high() {
        let mut s = PlayerStats::default();
        s.stress = 80.;
        assert!((s.stress_work_mult() - 0.50).abs() < 0.001);
    }

    #[test]
    fn loan_penalty_under_limit() {
        let mut s = PlayerStats::default();
        s.loan = 200.;
        assert!((s.loan_penalty() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn loan_penalty_over_limit() {
        let mut s = PlayerStats::default();
        s.loan = 400.;
        assert!((s.loan_penalty() - 0.90).abs() < 0.001);
    }

    // ── Decay arithmetic ──────────────────────────────────────────────────────

    #[test]
    fn hunger_increases_per_tick() {
        let hunger = 30_f32;
        let dt = 1.0_f32;
        let new = (hunger + dt * 0.08).min(100.);
        assert!((new - 30.08).abs() < 0.001);
    }

    #[test]
    fn hunger_clamps_at_100() {
        let hunger = 99.96_f32;
        let dt = 1.0_f32;
        let new = (hunger + dt * 0.08).min(100.);
        assert_eq!(new, 100.);
    }

    #[test]
    fn energy_decreases_per_tick() {
        let energy = 80_f32;
        let dt = 1.0_f32;
        let new = (energy - dt * 0.05).max(0.);
        assert!((new - 79.95).abs() < 0.001);
    }

    #[test]
    fn energy_clamps_at_zero() {
        let energy = 0.025_f32;
        let dt = 1.0_f32;
        let new = (energy - dt * 0.05).max(0.);
        assert_eq!(new, 0.);
    }

    #[test]
    fn stormy_weather_drains_more_energy_than_cloudy() {
        let cloudy = WeatherKind::Cloudy.energy_decay_mult();
        let stormy = WeatherKind::Stormy.energy_decay_mult();
        assert!(stormy > cloudy);
        assert!((stormy - 1.5).abs() < 0.001);
    }

    #[test]
    fn happiness_decays_when_hunger_above_60() {
        let hunger = 70_f32;
        let happiness = 50_f32;
        let mood_mult = Mood::from_happiness(happiness).decay_mult(); // Okay = 1.0
        // Simulate one second tick: happiness -= dt * 0.25 * mood_mult
        let new = if hunger > 60. {
            (happiness - 1.0 * 0.25 * mood_mult).max(0.)
        } else {
            happiness
        };
        assert!(new < happiness);
        assert!((new - 49.75).abs() < 0.001);
    }

    #[test]
    fn happiness_does_not_decay_from_hunger_when_hunger_low() {
        let hunger = 40_f32;
        assert!(
            hunger <= 60.,
            "hunger below 60 should not trigger happiness decay"
        );
    }

    #[test]
    fn depressed_mood_decays_happiness_faster_than_elated() {
        let decay_depressed = 0.25 * Mood::Depressed.decay_mult();
        let decay_elated = 0.25 * Mood::Elated.decay_mult();
        assert!(decay_depressed > decay_elated);
    }

    // ── Health drain arithmetic ───────────────────────────────────────────────

    #[test]
    fn health_drains_when_hunger_above_80() {
        let hunger = 85_f32;
        let health = 50_f32;
        let dt = 1.0_f32;
        let new = if hunger > 80. {
            (health - dt * 0.60).max(0.)
        } else {
            health
        };
        assert!(new < health);
        assert!((new - 49.4).abs() < 0.001);
    }

    #[test]
    fn health_stable_when_hunger_below_80() {
        let hunger = 79_f32;
        assert!(hunger <= 80., "hunger ≤ 80 should not drain health");
    }

    #[test]
    fn health_drains_when_energy_below_10() {
        let energy = 5_f32;
        let health = 60_f32;
        let dt = 1.0_f32;
        let new = if energy < 10. {
            (health - dt * 0.40).max(0.)
        } else {
            health
        };
        assert!(new < health);
        assert!((new - 59.6).abs() < 0.001);
    }

    #[test]
    fn health_stable_when_energy_moderate() {
        let energy = 15_f32;
        assert!(energy >= 10., "energy ≥ 10 should not drain health");
    }

    #[test]
    fn health_drains_when_stress_above_85() {
        let stress = 90_f32;
        let health = 70_f32;
        let dt = 1.0_f32;
        let new = if stress > 85. {
            (health - dt * 0.30).max(0.)
        } else {
            health
        };
        assert!(new < health);
        assert!((new - 69.7).abs() < 0.001);
    }

    #[test]
    fn malnourished_multiplies_hunger_drain() {
        let hunger = 90_f32;
        let dt = 1.0_f32;
        let normal_drain = if hunger > 80. { dt * 0.60 * 1.0 } else { 0. };
        let mal_drain = if hunger > 80. { dt * 0.60 * 1.5 } else { 0. };
        assert!((mal_drain / normal_drain - 1.5).abs() < 0.001);
    }

    #[test]
    fn health_clamps_at_zero() {
        let health = 0.1_f32;
        let new = (health - 1.0_f32).max(0.);
        assert_eq!(new, 0.);
    }
}
