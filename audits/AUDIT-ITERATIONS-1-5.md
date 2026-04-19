# Audit Iterations 1-5

Iteration summaries for the [main audit document](AUDIT-SUMMARY.md).

---

### Iteration 1 - 2026-04-19

**Auditor:** Claude (Explore agent) + manual verification
**Scope:** Full codebase, all 24 `.rs` files
**Method:** Static analysis via file reads + pattern search

**Findings opened this iteration:**
R-01, R-02, R-03, R-04, R-05, I-01, I-02, I-03, I-04, I-05, T-01, T-02, T-03, T-04, T-05, S-01

**Findings closed/accepted this iteration:**
S-02 (no unsafe - accepted), S-03 (save.json plain text - accepted single-player)

**Agent finding corrected:**
The Explore agent reported `sample_save()` was missing `mental_fatigue`,
`high_stress_days`, and `low_stress_days`. Manual verification (save.rs lines
741-743) confirmed these fields are present. Finding dismissed.

**Top priorities for next session:**
1. T-01 - Extract pure business logic from `interaction.rs` and add unit tests
2. R-01 - Begin splitting `handle_interaction` into helper functions
3. I-01 - Remove unnecessary `.clone()` calls on `Handle` in `audio.rs`
4. S-01 - Route save failure through `Notification`

---

### Iteration 2 - 2026-04-19

**Auditor:** Claude (in-session remediation)
**Scope:** High-severity findings from Iteration 1: T-01, T-02, I-02, I-03
**Method:** Code edits + test additions

**Findings fixed this iteration:**
- **T-02** - Extracted `crisis_should_trigger(seed, base_chance, day, insured) -> bool` from `crisis_trigger_system`. Added 6 deterministic unit tests (roll boundaries, difficulty tiers, insurance reduction, day scaling, seed stability).
- **I-02** - `crisis.rs` `crisis_day_tick`: replaced `.unwrap()` with `let Some(kind) = ... else { return; }`.
- **I-03** - `interaction.rs` festival active unwraps at ~713 and ~887: converted to safe `let Some(...) else { return; }` and `if let Some(...)` patterns.
- **T-01 (partial)** - Extracted 4 pure helper functions from `handle_interaction`: `health_work_mult`, `freelance_base_pay`, `meal_tier`, `exercise_energy_cost`. Added 10 unit tests covering tiers and boundary values. ECS-dependent paths (bank, invest, housing) remain untested.

**Findings not addressed this iteration:**
- R-01 (split `handle_interaction`) - helper extraction is partial progress; full dispatcher split deferred
- I-02 partial - `festival.rs` line ~38 still uses `.unwrap()` on `festival.active.take()`
- I-01 - unnecessary clone in `audio.rs` (Medium, quick fix, next session)
- S-01 - save failure not player-visible (Medium)

**T-01 completed (same session follow-up):**
- `try_deposit` / `try_withdraw` extracted, wired into `handle_bank_input` and the p3 quick-deposit shortcut (also eliminated one `_m = take` antipattern instance)
- 10 bank-logic tests added to `interaction.rs` (deposit/withdraw success, exact-amount, insufficient funds, zero/negative amount)
- 20 tests added to `resources.rs` covering every previously untested method feeding work/bank/housing calculations: `Reputation::work_mult` (tiers + boundaries), `Reputation::chat_bonus`, `Skills::work_pay` (career tiers + all streak boundaries), `PlayerStats::stress_work_mult`/`loan_penalty`/`skill_gain_mult`, `HousingTier::has_access`/`upgrade_cost`/`next`/`rent`, `Conditions::work_pay_mult` stacking (burnout x malnourished x mental_fatigue), `Skills::cooking_bonus`/`social_bonus`/`career_rank`
- T-01 closed

**Top priorities for next session:**
1. Fix remaining `festival.rs` `.unwrap()` (I-02 remainder - 5 min)
2. Continue R-03 cleanup: remaining `_m = take` patterns in bank/transport shortcuts
3. R-01: split `handle_interaction` into category helpers

---

### Iteration 3 - 2026-06-16

**Auditor:** Claude (in-session remediation)
**Scope:** Medium-severity findings: I-04, S-01, R-04 (player.rs only), R-05, I-05, I-01
**Method:** Code edits + build verification

**Findings fixed this iteration:**
- **I-04** - Added `From<u8>` / `From<&Enum> for u8` traits on `HousingTier`, `TransportKind`, `PetKind`. Added `CrisisKind::from_u8(u8) -> Option<CrisisKind>` (orphan rule prevents `From<u8> for Option<CrisisKind>`). Replaced all 8 manual match blocks in `handle_save` and `apply_save_data`.
- **S-01** - `handle_save` now takes `ResMut<Notification>` and pushes player-visible messages on save/serialize failure.
- **R-04 (player.rs)** - Moved `ACCEL`, `FRICTION`, `SPRINT_MULT`, `SPRINT_DRAIN`, boundary `1600` to `constants.rs` as named constants. `crisis.rs` magic numbers deferred (other agent working on that file).
- **R-05** - Removed dead `HobbyKind::label()` method and `#[allow(dead_code)]`.
- **I-05** - Added explanatory comment on friendship `Vec` collect in `time.rs`.

**Findings accepted this iteration:**
- **I-01** - `Handle<AudioSource>` is `Clone` not `Copy` in Bevy 0.15. The `.clone()` calls are correct.

**Top priorities for next session:**
1. R-04 remainder - `crisis.rs` magic numbers (when other agent finishes)
2. I-02 remainder - `festival.rs` `.unwrap()` on `festival.active.take()`
3. R-01 - Continue splitting `handle_interaction` into helper functions
4. T-01 - Add more pure helper tests for bank/invest/housing paths

---

### Iteration 4 - 2026-04-19

**Auditor:** Claude (in-session remediation)
**Scope:** R-03, I-02 remainder, T-03, T-05, S-01 remainder (settings.rs)
**Method:** Code edits + test additions + build verification

**Planned changes:**
- **R-03** - Add `Notification::extend_timer(dur)` method to resources.rs. Replace all 17 `std::mem::take` + `notif.push` sites in interaction.rs with `notif.extend_timer(dur)`.
- **I-02 remainder** - `festival.rs` line 38: replace `if festival.active.is_some() { let kind = festival.active.take().unwrap()` with `if let Some(kind) = festival.active.take()`.
- **T-03** - Extract pure `compute_goal_progress` helper from `check_daily_goal` in goals.rs; add unit tests for goal progress calculations.
- **T-05** - Add sprint drain and boundary clamp tests to player.rs.
- **S-01 remainder** - `settings.rs` lines 115/118: add `ResMut<Notification>` and push notification on config write/serialize failure.

**Findings fixed this iteration:**
- **R-03** - Added `Notification::flush_message(dur)` method to resources.rs. Replaced all 17 `std::mem::take` + `notif.push` sites in interaction.rs with `notif.flush_message(dur)`.
- **I-02 remainder** - `festival.rs` line 38: replaced `if festival.active.is_some() { let kind = festival.active.take().unwrap()` with `if let Some(kind) = festival.active.take()`.
- **T-03** - Extracted pure `compute_goal_progress` function from `check_daily_goal` in goals.rs. Added 15 unit tests covering EarnMoney, SaveMoney, LowerStress, FeedPet, OutdoorWeather, SeasonalGoal, MaintainHappy, and WorkTimes.
- **T-05** - Extracted `compute_max_speed`, `clamp_position`, `sprint_drain` pure helpers from `player_movement` in player.rs. Added 11 unit tests covering speed multipliers, boundary clamping, and sprint energy drain.
- **S-01 remainder** - `GameSettings::save()` now returns `bool`. Added `ResMut<Notification>` to `handle_menu_buttons` in menu.rs; pushes notification on save failure at all 3 call sites.

**Top priorities for next session:**
1. R-01 - Split `handle_interaction` into helper functions (High severity, deferred)
2. R-02 - Split `on_new_day` into helper functions (Medium severity, deferred)
3. R-04 remainder - `crisis.rs` magic numbers
4. T-04 - `sample_save()` sync checklist
