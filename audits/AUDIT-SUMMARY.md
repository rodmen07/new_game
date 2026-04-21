# new_game — Code Audit Process

This document is the living record of code quality audits for `d:\Projects\new_game\`.
Re-run the process below at any time to refresh findings. Each iteration appends a new
entry to the Iteration Log at the bottom.

---

## How to Run an Audit

For the full agentic workflow, see [AGENT-AUDIT-PROMPT.md](AGENT-AUDIT-PROMPT.md).
For the complete list of audit dimensions, see [AUDIT-DIMENSIONS.md](AUDIT-DIMENSIONS.md).

### 1. Automated checks (run in VS Code terminal)

```bash
# Compile-time lints — zero tolerance
cargo clippy -- -D warnings

# Tests
cargo test

# Count untested surface area
grep -rn "#\[test\]" src/ | wc -l          # test count
grep -rn "pub fn " src/ | wc -l            # public fn count
```

### 2. Manual review checklist

For each file in `src/`:

- [ ] Any function longer than ~150 lines?
- [ ] Any repeated code block appearing 3+ times?
- [ ] Any `.unwrap()` / `.expect()` outside of tests?
- [ ] Any `.clone()` on a `Copy` type?
- [ ] Any `eprintln!` error that should reach the player as a notification?
- [ ] Any new `SaveData` field missing from `sample_save()` in `save.rs`?
- [ ] Any new resource field missing from serialize/deserialize in `save.rs`?
- [ ] Any magic number that should be a named constant?

### 3. Test coverage check

```bash
# Files with zero #[test] functions
for f in src/**/*.rs; do
  grep -q "#\[test\]" "$f" || echo "NO TESTS: $f"
done
```

### 4. After reviewing, append findings below

Use the Finding template and the Iteration Log format shown at the bottom.

---

## Finding Reference

Each finding has:

- **ID** — stable identifier across iterations
- **File** — path relative to `src/`
- **Lines** — approximate range (update if file shifts)
- **Severity** — High / Medium / Low
- **Category** — Readability | Idiomatic | Coverage | Safety
- **Status** — Open | Fixed | Accepted (with reason)
- **Description + Suggestion**

---

## Category: Readability / Maintainability

### R-01 — `handle_interaction` is a 1,400+ line match

| Field | Value |
|---|---|
| File | `systems/interaction.rs` |
| Lines | ~51-1458 |
| Severity | High |
| Status | **Fixed (Iteration 6, 2026-06-16)** - Iteration 5: 3 pre-match shortcut blocks (~260 lines) extracted. Iteration 6: 5 match arms (Work, Shop, Relax/Festival, Chat, StudyCourse) extracted to helper functions. Match arms now dispatch to one-line helper calls. |

`handle_interaction` contains 24+ `ActionKind` match arms, each with substantial
business logic. The function is the single largest in the codebase and is a friction
point for every mechanic addition.

**Suggestion:** Extract each logical group into a helper (e.g., `handle_work`,
`handle_finance`, `handle_social`, `handle_shop`). The outer match stays as a
dispatcher; each arm is one line calling a helper. No behavior change needed.

---

### R-02 — `on_new_day` handles 20+ daily mechanics inline

| Field | Value |
|---|---|
| File | `systems/time.rs` |
| Lines | ~400–750 |
| Severity | Medium |
| Status | **Fixed (Iteration 6, 2026-06-16)** - 7 helpers extracted: `tick_conditions`, `tick_investments`, `apply_rent`, `decay_friendships`, `decay_skills`, `reset_daily_state`, `apply_daily_event`. `on_new_day` now reads as a sequential dispatcher. |

Every daily mechanic (rent, loans, skill decay, conditions, quests, friendship,
seasons, weather) runs sequentially inside one function. Hard to locate a specific
mechanic or audit ordering.

**Suggestion:** Extract into `apply_daily_rent`, `tick_conditions`, `decay_skills`,
`tick_investments`, etc., called in sequence from `on_new_day`.

---

### R-03 — Repeated `std::mem::take` pattern

| Field | Value |
|---|---|
| File | `systems/interaction.rs` |
| Lines | ~130, 145, 161, 182, 201, 218, 235, 259, 277, 303 |
| Severity | Medium |
| Status | **Fixed (Iteration 4, 2026-06-16)** |

The idiom `let _m = std::mem::take(&mut notif.message); notif.push(_m, dur);`
appears 10+ times. The name `_m` is opaque and the pattern is boilerplate.

**Suggestion:** Add a method to `Notification`: `fn extend_timer(&mut self, dur: f32)`
that does the take-and-re-push in one place.

---

### R-04 — Magic numbers in crisis and player systems

| Field | Value |
|---|---|
| Files | `systems/crisis.rs` ~32–54, `systems/player.rs` ~7–10, 67–68 |
| Severity | Medium |
| Status | **Fixed (Iteration 5, 2026-06-16)** - Player constants fixed. All crisis.rs magic numbers extracted to constants.rs (20+ named constants). |

`constants.rs` already exists. These numbers should live there with doc comments
explaining their gameplay meaning.

---

### R-05 — `HobbyKind::label()` is dead code

| Field | Value |
|---|---|
| File | `components.rs` |
| Lines | ~78–79 |
| Severity | Low |
| Status | **Fixed (2026-06-16)** — `HobbyKind::label()` and `#[allow(dead_code)]` removed. |

---

## Category: Idiomatic Rust

### I-01 — Unnecessary `.clone()` on `Copy` types in `audio.rs`

| Field | Value |
|---|---|
| File | `audio.rs` |
| Lines | ~99, 139, 154, 172–176 |
| Severity | Medium |
| Status | **Accepted (2026-06-16)** — `Handle<AudioSource>` is `Clone` not `Copy` in Bevy 0.15. The `.clone()` calls are correct and necessary. |

**Suggestion:** Remove all `.clone()` calls on `Handle` fields. Use them directly
or with copy semantics.

---

### I-02 — `.unwrap()` after external guard in `crisis.rs` and `festival.rs`

| Field | Value |
|---|---|
| Files | `systems/crisis.rs` ~180, `systems/festival.rs` ~38 |
| Severity | Medium |
| Status | **Fixed (Iteration 4, 2026-06-16)** — `crisis.rs` fixed (2026-04-19). `festival.rs` fixed with `if let Some(kind)` pattern. |

```rust
// crisis.rs — after fix
let Some(kind) = crisis.active.take() else { return; };
```

---

### I-03 — `.unwrap()` on festival active in `interaction.rs`

| Field | Value |
|---|---|
| File | `systems/interaction.rs` |
| Lines | ~713, ~887 |
| Severity | Medium |
| Status | **Fixed (2026-04-19)** — both sites converted: `let Some(kind) = ... else { return; }` and `if let Some(k) = ... { ... }`. |

---

### I-04 — Manual enum ↔ `u8` conversions in `save.rs`

| Field | Value |
|---|---|
| File | `save.rs` |
| Lines | ~324–329, 376–407, 479–484, 547–595 |
| Severity | Medium |
| Status | **Fixed (2026-06-16)** — `From<u8>` / `From<&Enum> for u8` traits added for `HousingTier`, `TransportKind`, `PetKind`; `CrisisKind::from_u8(u8) -> Option<CrisisKind>` added. Both `handle_save` and `apply_save_data` now use trait calls instead of manual matches. |

**Suggestion:** Implement `From<HousingTier> for u8` and `From<u8> for HousingTier`
(or use `repr(u8)` with a `TryFrom`) on each enum. Save/load then become single
`.into()` / `TryFrom::try_from()` calls.

---

### I-05 — Intermediate `Vec` in friendship decay loop

| Field | Value |
|---|---|
| File | `systems/time.rs` |
| Lines | ~701–712 |
| Severity | Low |
| Status | **Fixed (2026-06-16)** — Added `// collect first: can't mutate friendship while iterating its keys` comment. |
Fine at 6 NPCs; worth a comment so future readers don't think the collect is
unnecessary.

**Suggestion:** Add a `// collect first: can't mutate `friendship` while iterating its keys`
comment.

---

## Category: Test Coverage

### T-01 — `systems/interaction.rs` has zero tests

| Field | Value |
|---|---|
| File | `systems/interaction.rs` |
| Lines | entire file (~1,580 lines after changes) |
| Severity | High |
| Status | **Fixed (2026-04-19)** — see detail below. |

**What was done:**
- 6 pure helpers extracted and tested in `interaction.rs`: `health_work_mult`, `freelance_base_pay`, `meal_tier`, `exercise_energy_cost`, `try_deposit`, `try_withdraw` — 20 unit tests total.
- `try_deposit`/`try_withdraw` wired into `handle_bank_input` and the p3 quick-deposit shortcut (also fixed one `_m = take` instance).
- 20 additional tests added to `resources.rs` covering every previously untested method that feeds the work/bank/housing calculation: `Reputation::work_mult` (tiers + boundaries), `Reputation::chat_bonus`, `Skills::work_pay` (tiers + streak boundaries), `PlayerStats::stress_work_mult`/`loan_penalty`/`skill_gain_mult`, `HousingTier::has_access`/`upgrade_cost`/`next`/`rent`, `Conditions::work_pay_mult` stacking (burnout × malnourished × mental_fatigue), `Skills::cooking_bonus`/`social_bonus`, `Skills::career_rank`.

ECS-dependent paths (invest, crafting item counts, housing-upgrade gate) remain untested — these require a Bevy `World` and are the natural next target if integration tests are added.

---

### T-02 — `systems/crisis.rs` has zero tests

| Field | Value |
|---|---|
| File | `systems/crisis.rs` |
| Severity | High |
| Status | **Fixed (2026-04-19)** — `crisis_should_trigger` extracted as a pure fn; 6 tests added covering roll boundaries, difficulty scaling, insurance reduction, day-based threshold growth, and determinism. |

---

### T-03 — `systems/goals.rs` has zero tests

| Field | Value |
|---|---|
| File | `systems/goals.rs` |
| Severity | Medium |
| Status | **Fixed (Iteration 4, 2026-06-16)** |

Goal completion checks and reward grants are untested. Goals affect the player's
daily direction significantly.

---

### T-04 — `sample_save()` must stay in sync with `SaveData` fields

| Field | Value |
|---|---|
| File | `save.rs` |
| Lines | ~679–779 |
| Severity | Medium |
| Status | **Accepted (Iteration 5, 2026-06-16)** - Process rule with checklist comment added to save.rs. |

`sample_save()` is a full struct literal - it will fail to compile if a new field
is added to `SaveData` without adding a corresponding line. This is actually a
good property (compile-time completeness check), but it means every new field
requires two edits: struct definition and `sample_save()`. The process to do this
should be part of the "new SaveData field" checklist.

**Checklist for adding a new `SaveData` field:**
1. Add field + `#[serde(default)]` to `SaveData` struct
2. Add serialize line in `handle_save` (`data.field = resource.field`)
3. Add deserialize line in `apply_save_data` (`resource.field = data.field`)
4. Add field to `sample_save()` in tests

---

### T-05 — `systems/player.rs` boundary conditions untested

| Field | Value |
|---|---|
| File | `systems/player.rs` |
| Severity | Low |
| Status | **Fixed (Iteration 4, 2026-06-16)** |

Sprint drain, energy clamping, and position boundary clamping have no tests.

---

## Category: Safety / Reliability

### S-01 — Save and settings failures are silent to the player

| Field | Value |
|---|---|
| Files | `save.rs` ~420–430, `settings.rs` ~114–119 |
| Severity | Medium |
| Status | **Fixed (Iteration 4, 2026-06-16)** — `save.rs` fixed: `handle_save` pushes notifications on failure. `settings.rs` fixed: `save()` returns `bool`, menu notifies player on failure. |

**Suggestion:** Emit a `Notification` event on save failure:
```rust
Err(e) => {
    eprintln!("[save] Write failed: {e}");
    notif.push("Warning: save failed! Progress may be lost.", 8.);
}
```
Requires `save_system` to receive `ResMut<Notification>`.

---

### S-02 — No `unsafe` code

| Field | Value |
|---|---|
| Status | **Accepted — no action needed** |

Full codebase scan found zero `unsafe` blocks.

---

### S-03 — Bank input is bounded; save JSON is unencrypted

| Field | Value |
|---|---|
| File | `systems/interaction.rs` ~1495, `save.rs` |
| Severity | Low |
| Status | **Accepted — single-player game** |

Bank input is capped at 7 digits. Save JSON is plain text and can be hand-edited.
Both are acceptable for a local single-player game; no action required.

---

## Category: Multiplayer Readiness

### M-01 - Per-player state stored as global Resources

| Field | Value |
|---|---|
| Files | `resources.rs`, all `systems/*.rs` |
| Severity | High (Planning) |
| Status | **Fixed (current session)** - All 5 remaining per-player types (`PlayerStats`, `Inventory`, `Skills`, `WorkStreak`, `HousingTier`) migrated to Components. All systems updated to use `Query<..., With<LocalPlayer>>`. |

`PlayerId` and `LocalPlayer` components added in earlier iterations. This session
completed the migration by converting `PlayerStats`, `Inventory`, `Skills`,
`WorkStreak`, and `HousingTier` from `#[derive(Resource)]` to `#[derive(Component)]`.
All affected systems (time.rs, interaction.rs, and others) updated to use
`Query<..., With<LocalPlayer>>`. `DayExtras.inv` field removed; inventory access
consolidated into the player query tuple in `on_new_day`.

---

### M-02 - `get_single_mut()` assumes one player

| Field | Value |
|---|---|
| Files | `systems/player.rs`, `systems/collision.rs`, `systems/vehicle.rs`, `systems/time.rs`, `systems/npc.rs`, `systems/visual.rs` |
| Lines | 14 call sites |
| Severity | High (Planning) |
| Status | **Open** |

14 calls to `get_single()` / `get_single_mut()` on player and camera queries
will panic if a second player entity exists. Each needs to become a filtered
query or an iterator over all players.

---

### M-03 - Input is a global singleton

| Field | Value |
|---|---|
| Files | `systems/interaction.rs`, `systems/player.rs` |
| Severity | High (Planning) |
| Status | **Open** |

`Res<ButtonInput<KeyCode>>` is polled directly. No input mapping layer,
no player-index routing. Multiplayer needs an action-event abstraction
(e.g., `PlayerAction { player_id, action }`) or `bevy_replicon` input replication.

---

### M-04 - SaveData is single-player flat struct

| Field | Value |
|---|---|
| File | `save.rs` |
| Severity | Medium (Planning) |
| Status | **Open** |

`SaveData` mirrors all per-player resources as flat fields. No player ID,
no array of players. Multiplayer needs either `Vec<PlayerSave>` or separate
save files per player with a world-state file.

---

### M-05 - SystemParam bundles at 16-field maximum

| Field | Value |
|---|---|
| File | `resources.rs` |
| Severity | Medium (Planning) |
| Status | **Fixed (current session)** - `on_new_day` consolidated 4 separate `ResMut` params plus `Inventory` into one 5-tuple `Query`, keeping total system params within the 16-field limit. |

`InteractExtras` was reduced from 16 to 13 fields in iterations 7-8. This session
resolved remaining pressure by consolidating `PlayerStats`, `Skills`, `WorkStreak`,
`HousingTier`, and `Inventory` into a single `Query` tuple in `on_new_day`, removing
4 separate `ResMut` params and staying well within the Bevy 16-param limit.

---

### R-06 — `setup()` is a 780-line world-spawn function

| Field | Value |
|---|---|
| File | `setup.rs` |
| Lines | ~114-893 |
| Severity | High |
| Status | **Fixed (current session)** - `setup()` now delegates to 7 private helpers + `spawn_hud`. |

Extracted: `spawn_terrain_and_roads`, `spawn_buildings_and_zones`, `spawn_vehicle`,
`spawn_world_objects`, `spawn_npcs`, `spawn_player_entity`, and
`spawn_collision_walls_and_roads`. `setup()` is now 12 lines. All helpers take
`commands: &mut Commands` and compile cleanly.

---

### R-07 — `spawn_hud()` is 375 lines

| Field | Value |
|---|---|
| File | `setup.rs` |
| Lines | ~1057-1431 |
| Severity | High |
| Status | **Fixed (current session)** - `spawn_hud()` now delegates to 4 private `ChildBuilder` helpers. |

Extracted: `spawn_hud_left_panel`, `spawn_hud_right_panel`,
`spawn_hud_notification_area`, and `spawn_hud_prompt_overlay`. Each takes
`root: &mut ChildBuilder`. `spawn_hud` is now 14 lines. Compiles cleanly.

---

### R-08 — `build_prompt_challenge()` is 242 lines

| Field | Value |
|---|---|
| File | `systems/interaction.rs` |
| Lines | ~118-359 |
| Severity | Medium |
| Status | **Fixed (current session)** - `build_prompt_challenge()` now delegates to 8 focused helper functions. |

Extracted: `action_challenge`, `item_challenge`, `hobby_challenge`,
`social_challenge`, `finance_challenge`, `transport_challenge`, `craft_challenge`,
and `festival_challenge`. Module-level constants (`OFFICE_WORDS`, `FOOD_WORDS`,
etc.) and a `pick_word()` free function replace the inline closure. `build_prompt_challenge` is now 13 lines. Compiles cleanly.

---

### R-09 — `make_goal()` is 185 lines of repeated struct literals

| Field | Value |
|---|---|
| File | `systems/time.rs` |
| Lines | ~956-1140 |
| Severity | Medium |
| Status | **Fixed (Iteration 13, 2026-04-19)** - Extracted `goal()` constructor helper. `make_goal()` reduced from 185 lines to 20 lines. All values preserved. Existing 8 tests continue to pass. |

The same 8-field `DailyGoal` struct literal was repeated 18 times with only 5 fields varying per arm (`kind`, `description`, `target`, `reward_money`, `reward_happiness`). `progress`, `completed`, and `failed` were always the same defaults.

**Suggestion:** Extract a `goal(kind, desc, target, money, hap) -> DailyGoal` helper and convert the match arms to one-line calls.

---

## Iteration Log

Each audit run appends a dated entry to the iteration files below.
Each file holds up to 5 iterations.

- [Iterations 1-5](AUDIT-ITERATIONS-1-5.md) (5 of 5 used)
- [Iterations 6-12](AUDIT-ITERATIONS-6-10.md) (7 entries - overflowed; next entries in 11-15 file)
- [Iterations 13-15](AUDIT-ITERATIONS-11-15.md) (1 of 5 used)

_Template for future entries:_

```
### Iteration N - YYYY-MM-DD

**Auditor:**
**Scope:**
**Method:**

**Findings opened:**

**Findings closed/accepted:**

**Changes made:**

**Top priorities for next session:**
```
