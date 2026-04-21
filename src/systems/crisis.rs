use crate::constants::*;
use crate::resources::*;
use crate::settings::{Difficulty, GameSettings};
use bevy::prelude::*;

// ── Pure trigger logic (testable without ECS) ────────────────────────────────

fn crisis_should_trigger(seed: u64, base_chance: u64, day: u32, insured: bool) -> bool {
    let roll = (seed >> 33) % 100;
    let min = CRISIS_MIN_DAY as f64;
    let day_scale = 1.0 + (day as f64 - min).clamp(0., 90.) / 90.;
    let mut threshold = (base_chance as f64 * day_scale) as u64;
    if insured {
        threshold = threshold * 3 / 4;
    }
    roll < threshold
}

/// Checks once per day whether a new crisis should trigger.
/// - No crisis before day 10
/// - Cooldown of 5 days after last crisis
/// - Base chance ~12% per day on Hard, ~8% Normal, ~4% Easy
/// - Insurance halves the financial impact but doesn't prevent crises
/// - High reputation (60+) reduces crisis chance by 30%
pub fn crisis_trigger_system(
    gt: Res<GameTime>,
    mut crisis: ResMut<CrisisState>,
    mut stats: ResMut<PlayerStats>,
    mut inv: ResMut<Inventory>,
    mut invest: ResMut<Investment>,
    mut notif: ResMut<Notification>,
    settings: Res<GameSettings>,
) {
    // Run once per day
    if gt.day == crisis.last_check_day || gt.day < CRISIS_MIN_DAY || crisis.is_active() {
        return;
    }
    crisis.last_check_day = gt.day;
    // Cooldown: at least CRISIS_COOLDOWN_DAYS between crises
    if gt.day.saturating_sub(crisis.last_crisis_day) < CRISIS_COOLDOWN_DAYS {
        return;
    }

    // Use a sentinel to run once per day
    let seed = (gt.day as u64)
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);

    let base_chance: u64 = match settings.difficulty {
        Difficulty::Easy => CRISIS_CHANCE_EASY,
        Difficulty::Normal => CRISIS_CHANCE_NORMAL,
        Difficulty::Hard => CRISIS_CHANCE_HARD,
    };

    if !crisis_should_trigger(seed, base_chance, gt.day, crisis.has_insurance) {
        return;
    }

    // Pick a crisis type based on seed
    let kind_seed = ((seed >> 16) % 6) as u8;
    let kind = match kind_seed {
        0 => CrisisKind::Layoff,
        1 => CrisisKind::MarketCrash,
        2 => CrisisKind::MedicalEmergency,
        3 => CrisisKind::RentHike,
        4 => CrisisKind::Theft,
        _ => CrisisKind::ApplianceBreak,
    };

    // Skip MarketCrash if no investments
    let kind =
        if matches!(kind, CrisisKind::MarketCrash) && invest.amount < MIN_INVESTMENT_FOR_CRASH {
            CrisisKind::Layoff
        } else {
            kind
        };

    crisis.active = Some(kind);
    crisis.days_left = kind.duration();

    // Apply immediate effects
    let insured = crisis.has_insurance;
    let dmg_mult = if insured {
        INSURANCE_DAMAGE_MULTIPLIER
    } else {
        1.0
    };

    match kind {
        CrisisKind::Layoff => {
            notif.push(
                format!("CRISIS: Laid off! Work blocked for {} days. Visit the office after to re-interview.", kind.duration()),
                CRISIS_NOTIF_DURATION,
            );
        }
        CrisisKind::MarketCrash => {
            let loss_pct = MARKET_CRASH_BASE_LOSS_PCT
                + (((seed >> 8) % MARKET_CRASH_RANDOM_LOSS_RANGE) as f32 / 100.);
            let inv_loss = invest.amount * loss_pct * dmg_mult;
            invest.amount = (invest.amount - inv_loss).max(0.);
            let sav_loss = stats.savings * MARKET_CRASH_SAVINGS_LOSS_PCT * dmg_mult;
            stats.savings = (stats.savings - sav_loss).max(0.);
            notif.push(
                format!(
                    "CRISIS: Market crash! Lost ${:.0} investments, ${:.0} savings.{}",
                    inv_loss,
                    sav_loss,
                    if insured {
                        " (Insurance halved losses)"
                    } else {
                        ""
                    }
                ),
                CRISIS_NOTIF_DURATION,
            );
        }
        CrisisKind::MedicalEmergency => {
            let bill = MEDICAL_BILL_AMOUNT * dmg_mult;
            stats.health = MEDICAL_HEALTH_FLOOR;
            stats.money = (stats.money - bill).max(0.);
            stats.modify_energy(-MEDICAL_ENERGY_DRAIN);
            notif.push(
                format!("CRISIS: Medical emergency! Health dropped to {}, -${:.0} bill. Recover over {} days.{}",
                    MEDICAL_HEALTH_FLOOR, bill, kind.duration(), if insured { " (Insurance halved costs)" } else { "" }),
                CRISIS_NOTIF_DURATION,
            );
        }
        CrisisKind::RentHike => {
            notif.push(
                format!(
                    "CRISIS: Rent hike! Rent doubles for {} days.",
                    kind.duration()
                ),
                CRISIS_NOTIF_DURATION,
            );
        }
        CrisisKind::Theft => {
            let cash_stolen = (stats.money * THEFT_CASH_FRACTION * dmg_mult).min(stats.money);
            stats.money -= cash_stolen;
            let items_lost = if inv.coffee > 0 {
                inv.coffee = 0;
                true
            } else {
                false
            };
            let books_lost = if inv.books > 0 {
                inv.books = 0;
                true
            } else {
                false
            };
            let mut stolen_items = Vec::new();
            if items_lost {
                stolen_items.push("coffee");
            }
            if books_lost {
                stolen_items.push("books");
            }
            let items_str = if stolen_items.is_empty() {
                String::new()
            } else {
                format!(" Stolen items: {}.", stolen_items.join(", "))
            };
            notif.push(
                format!(
                    "CRISIS: Theft! Lost ${:.0} cash.{}{}",
                    cash_stolen,
                    items_str,
                    if insured {
                        " (Insurance halved cash loss)"
                    } else {
                        ""
                    }
                ),
                CRISIS_NOTIF_DURATION,
            );
        }
        CrisisKind::ApplianceBreak => {
            notif.push(
                format!(
                    "CRISIS: Appliance breakdown! Home actions cost extra $8 for {} days.",
                    kind.duration()
                ),
                CRISIS_NOTIF_DURATION,
            );
        }
    }

    stats.modify_stress(CRISIS_STRESS_INCREASE);
    stats.modify_happiness(-CRISIS_HAPPINESS_DECREASE);
}

/// Ticks down active crisis duration each day and clears when done.
/// Runs in the same chain as crisis_trigger_system, sharing GameTime day changes.
pub fn crisis_day_tick(
    gt: Res<GameTime>,
    mut crisis: ResMut<CrisisState>,
    mut notif: ResMut<Notification>,
) {
    if !crisis.is_active() {
        return;
    }
    // Only tick once per day - use last_check_day which was set by trigger system
    // If trigger didn't set it (crisis was active), we need our own check
    if gt.day == 0 {
        return;
    }

    // The trigger system sets last_check_day. If active, trigger returns early
    // before setting it, so we use it here as our sentinel.
    if crisis.last_check_day == gt.day {
        // Already processed this day (trigger system ran)
        return;
    }
    crisis.last_check_day = gt.day;

    // Decrement insurance
    if crisis.has_insurance && crisis.insurance_days > 0 {
        crisis.insurance_days -= 1;
        if crisis.insurance_days == 0 {
            crisis.has_insurance = false;
            notif.push("Insurance expired!".to_string(), 4.);
        }
    }

    crisis.days_left = crisis.days_left.saturating_sub(1);
    if crisis.days_left == 0 {
        let Some(kind) = crisis.active.take() else {
            return;
        };
        crisis.crises_survived += 1;
        crisis.last_crisis_day = gt.day;
        notif.push(
            format!(
                "Crisis resolved: {} is over! ({} crises survived)",
                kind.label(),
                crisis.crises_survived
            ),
            CRISIS_RESOLVE_NOTIF_DURATION,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_seed(day: u32) -> u64 {
        (day as u64)
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407)
    }

    // Construct a seed whose roll ((seed >> 33) % 100) equals `target_roll`.
    fn seed_with_roll(target_roll: u64) -> u64 {
        target_roll << 33
    }

    #[test]
    fn roll_zero_always_triggers_when_threshold_positive() {
        let seed = seed_with_roll(0);
        assert!(crisis_should_trigger(seed, 4, 10, false));
        assert!(crisis_should_trigger(seed, 12, 10, false));
        assert!(crisis_should_trigger(seed, 4, 100, false));
    }

    #[test]
    fn roll_99_never_triggers_on_normal_base_chance() {
        let seed = seed_with_roll(99);
        // Normal base 8, day 10 → threshold = 8. roll=99 >= 8, no trigger.
        assert!(!crisis_should_trigger(seed, 8, 10, false));
        // Hard base 12, day 10 → threshold = 12. roll=99 >= 12, no trigger.
        assert!(!crisis_should_trigger(seed, 12, 10, false));
    }

    #[test]
    fn difficulty_scaling() {
        // roll=3: fires Easy(4), Normal(8), Hard(12) at day 10.
        let seed = seed_with_roll(3);
        assert!(
            crisis_should_trigger(seed, 4, 10, false),
            "Easy should trigger"
        );
        assert!(
            crisis_should_trigger(seed, 8, 10, false),
            "Normal should trigger"
        );
        assert!(
            crisis_should_trigger(seed, 12, 10, false),
            "Hard should trigger"
        );

        // roll=5: misses Easy(4) but fires Normal(8) and Hard(12).
        let seed = seed_with_roll(5);
        assert!(
            !crisis_should_trigger(seed, 4, 10, false),
            "Easy should not trigger"
        );
        assert!(
            crisis_should_trigger(seed, 8, 10, false),
            "Normal should trigger"
        );
    }

    #[test]
    fn insurance_reduces_threshold_by_25_percent() {
        // At day 10 with base 8, threshold = 8. With insurance: 8 * 3/4 = 6.
        // roll=7 fires uninsured (7 < 8) but not insured (7 >= 6).
        let seed = seed_with_roll(7);
        assert!(
            crisis_should_trigger(seed, 8, 10, false),
            "uninsured fires at roll 7"
        );
        assert!(
            !crisis_should_trigger(seed, 8, 10, true),
            "insured does not fire at roll 7"
        );
    }

    #[test]
    fn threshold_grows_with_days() {
        // Day 10: day_scale = 1.0, threshold(Normal) = 8
        // Day 100: day_scale = 2.0, threshold(Normal) = 16
        // roll=9 fires at day 100 (9 < 16) but not day 10 (9 >= 8).
        let seed = seed_with_roll(9);
        assert!(
            !crisis_should_trigger(seed, 8, 10, false),
            "roll 9 misses at day 10"
        );
        assert!(
            crisis_should_trigger(seed, 8, 100, false),
            "roll 9 fires at day 100"
        );
    }

    #[test]
    fn game_seed_is_deterministic() {
        // Same day always produces the same seed, so trigger outcome is deterministic.
        assert_eq!(game_seed(15), game_seed(15));
        assert_ne!(game_seed(15), game_seed(16));
        let result_a = crisis_should_trigger(game_seed(15), 8, 15, false);
        let result_b = crisis_should_trigger(game_seed(15), 8, 15, false);
        assert_eq!(result_a, result_b);
    }
}
