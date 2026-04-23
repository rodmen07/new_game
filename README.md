# Everyday Life Simulator

A real-time 2D life management prototype written in Rust with Bevy. You guide a single character through work, meals, rent, sleep, friendships, hobbies, pets, weather, crises, and seasonal festivals across an accelerated city sandbox. Actions are gated by a full-screen typing challenge: a single random word displayed in large letters with per-character highlighting, auto-confirming the instant you finish typing.

**Play it now in your browser:** https://rodmen07.github.io/new_game/

---

## Overview

Genre: life sim / survival sandbox  
Perspective: top-down 2D  
Time scale: 60x (1 real second = 1 in-game minute)

The current playable build focuses on daily survival and long-term stability. You start with very little, work toward housing access, manage core needs, build friendships, survive bad luck, and slowly unlock stronger routines and milestone goals.

Current implemented highlights include:
- expanded HOME, LIBRARY, and OFFICE footprints with updated wall bounds and reorganized furniture layouts for improved room flow
- full-screen typing overlay: one random word per action, per-character green/highlight/gray feedback, auto-confirms on completion
- expanded word banks (15-20 words per action category, 35+ categories) for high variety
- save and load with JSON persistence
- Menu, Playing, Paused, and Settings screens
- banking, loans, investments, housing, and transport upgrades
- acquired pets and cars appear as visible world elements
- top-left HUD overlap reduced by moving/hiding the autoplay hint, and startup timeout now logs to console instead of showing a false in-game error banner
- NPC friendship, quests, narrative unlocks, and reputation systems
- pets, crisis events, seasonal festivals, and weather-driven visuals
- universal typed action prompts with seniority-based retries and subject-aware phrases before tasks resolve
- immediate apartment unlock when a bank deposit crosses the first housing threshold

The current baseline is verified with a successful build, a clean strict clippy run, and 172 passing tests.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust (Edition 2024) |
| Engine | [Bevy](https://bevyengine.org/) 0.15 |
| Architecture | Entity-Component-System (ECS) |
| Rendering | Bevy 2D sprite pipeline (no external renderer) |
| RNG | Linear congruential generator (seeded per day) |
| Dependencies | bevy 0.15 · bevy_tweening 0.12 · serde + serde_json · toml |

Bevy's ECS drives the entire game: most shared world state lives in Resources, while some player-local state now lives on Components. Game logic runs through scheduled Systems, with custom SystemParam groups helping keep larger systems manageable. `bevy_tweening` handles smooth HUD stat bar interpolation and the notification panel slide-in animation.

---

## Building & Running

### Prerequisites

- Rust stable toolchain: https://rustup.rs
- On Linux: libudev-dev, libasound2-dev, libxkbcommon-dev
- On Windows and macOS: no extra system packages are typically needed

### Common commands

```bash
cargo run
cargo run --release
cargo build
cargo test
```

The first build compiles the Bevy dependency tree and can take a few minutes. Windows debug builds are tuned in Cargo.toml for more stable linking during development.

### Browser build (primary)

The game is deployed to GitHub Pages and playable without any local setup:

https://rodmen07.github.io/new_game/

To run the browser build locally:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve --release
```

Then open http://127.0.0.1:8080 in a modern browser.

To build for deployment:

```bash
trunk build --release --public-url /new_game/
```

### Windows local troubleshooting

If the game appears stuck or Trunk reports Windows file/path errors such as `os error 32` or `os error 3`, this is usually a local file-lock race on `dist`/`wasm-target` from overlapping Trunk runs or background scanners.

Use the local helper:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\serve_local.ps1
```

Manual recovery sequence:

```powershell
Set-Location .\new_game
Get-Process trunk,wasm-opt,wasm-bindgen -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item .\dist -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item .\wasm-target -Recurse -Force -ErrorAction SilentlyContinue
trunk serve --release --port 8092 --dist .dist_local
```

Notes for the web build:
- game assets are served from the bundled assets directory
- settings and save progress live in browser localStorage instead of config.toml and save.json
- the canvas automatically fits the browser page
- startup diagnostics now wait up to 45 seconds and also detect `window.wasmBindings` so delayed wasm init does not produce a false startup timeout
- Bevy ECS on wasm will panic at startup if a system has overlapping `Query` access to the same component type (for example `Transform`), so queries must be made explicitly disjoint with `With`/`Without` filters
- Resolved: `resolve_collisions` now uses explicitly disjoint `Transform` queries (`LocalPlayer` + `Player` vs `Without<Player>` + `Without<LocalPlayer>`) to avoid Bevy `B0001` startup panic on web

---

## Controls

| Key | Action |
|---|---|
| W A S D / Arrow keys | Move |
| Shift (hold) | Sprint |
| E | Interact with the nearest object or NPC |
| G | Give a gift when available |
| 1-9 | Context actions such as shopping, banking, festival activities, and insurance |
| Enter | Confirm a typed action prompt (or it auto-confirms on word completion) |
| Backspace | Edit the active typed prompt |
| Esc | Pause, cancel a typed prompt, or access settings |
| Mouse wheel | Camera zoom |

Interaction prompts appear at the bottom of the HUD whenever you are in range of something useful. When you trigger an action, a full-screen overlay appears with a single word to type - characters turn green as you type, the current character is highlighted, and the action confirms automatically when the word is complete.

---

## Architecture

```text
src/
├── main.rs          # App entry point and system scheduling
├── menu.rs          # Main menu, pause menu, settings screen
├── audio.rs         # SFX and ambient audio plumbing
├── save.rs          # JSON persistence and reset flow
├── settings.rs      # config.toml load and save
├── components.rs    # ECS components including LocalPlayer and PlayerId
├── constants.rs     # Gameplay tuning constants
├── resources.rs     # Shared game resources and SystemParam bundles
├── setup.rs         # World, interactables, NPCs, and HUD spawn
└── systems/
    ├── player.rs
    ├── collision.rs
    ├── npc.rs
    ├── interaction.rs
    ├── stats.rs
    ├── time.rs
    ├── goals.rs
    ├── hud.rs
    ├── visual.rs
    ├── narrative.rs
    ├── vehicle.rs
    ├── crisis.rs
    └── festival.rs
```

- `TypingOverlay`, `TypingLabel`, `TypingWordTyped`, `TypingWordCurrent`, `TypingWordRemaining`, `TypingInstruction`, `TypingRetries` marker components for the typing challenge UI

### Key design notes

- Hybrid ECS state model: most global gameplay data still lives in Resources, while local player identity and several movement and input systems now live on the player entity as Components.
- Deterministic day-based randomness: wandering, events, and crisis timing use seeded logic for predictable testing with varied play.
- SystemParam bundles keep large Bevy systems readable while staying under the engine parameter limit.
- Collision is resolved in sub-steps to avoid tunneling at sprint and vehicle speeds.
- Save data mirrors the main gameplay state so runs can resume cleanly between sessions.

---

## Current Feature Set

### Survival loop
- Harder opening state where you begin with no lease and must earn access to stable housing
- Continuous management of energy, hunger, happiness, health, stress, and sleep debt
- Long-term condition systems including burnout, malnourishment, mental fatigue, and short hospitalization recovery
- Daily life rating from F through S based on how well the run is going

### Economy and progression
- Cash, savings, loan pressure, rent, eviction risk, and seeded investment returns
- Housing upgrades from unhoused to apartment, condo, and penthouse
- Transport progression from walking to bike and car, with movement and work bonuses
- Skills, hobbies, reputation, courses, and goal rewards feeding long-term progression
- 21 milestones including transport, social, crisis, quest, crafting, and festival achievements

### Social systems
- Three named NPCs with time-of-day routines and personality-based relationship bonuses
- Chatting, gifting, parties, study sessions, and friendship decay across days
- Quest board progression and narrative chapter unlocks tied to life progress
- Pet adoption and daily care with hunger tracking and passive bonuses

### Dynamic world systems
- Four seasons with rotating modifiers and seasonal daily goals
- Four weather patterns with particle effects, splash or snow variants, and storm flashes
- Six crisis types: layoff, market crash, medical emergency, rent hike, theft, and appliance breakdown
- Insurance purchase option at the bank to reduce crisis damage
- Seasonal festivals at the park with unique activities, token rewards, and a festival milestone

### Presentation and UX
- Main menu, pause flow, and in-game settings screen for difficulty and volume
- Full-screen typing challenge overlay: one random word per action drawn from 35+ category word banks (15-20 words each), with per-character green/highlight/gray feedback and auto-confirm on word completion - no Enter required
- Seniority-based retry count so skilled characters get fewer chances to fail
- Dual-panel HUD with goals, warnings, conditions, inventory, weather, story summary, and live interaction tips
- Smooth animated stat bars that lerp toward their real values instead of snapping, and a notification panel that slides in from above on new messages
- Optional ambient and weather audio that degrades gracefully if assets are missing
- JSON save and load support with day-start autosave behavior

---

## Verified Status

| Area | Status |
|---|---|
| Build | Passing |
| Clippy | Passing with cargo clippy -- -D warnings |
| Tests | 171 passing |
| Save and load | Implemented |
| Crisis system | Implemented |
| Seasonal festivals | Implemented |
| Settings screen | Implemented |
| Audio fallback | Implemented |
| Browser build | Live at rodmen07.github.io/new_game |
| Multiplayer support | Position sync with interpolation and stale-peer cleanup |

The project is currently a playable, feature-rich prototype with a clean verified Rust baseline and ongoing expansion work.

## Roadmap

Items are grouped by category and ordered by priority within each group. Each item lists its goal, the files it touches, and any prerequisite work.

---

### P1 - Test coverage for new systems

These are pure-function tests with no ECS dependency - low effort, high value.

**P1-A: Unit tests for the typing challenge helpers** ✅
- Pure-function tests added in `src/systems/interaction.rs`: `pick_word_*` (pool boundary, large-seed wrap, offset variation, determinism), `normalize_prompt_text_*` (lowercase, whitespace collapse, empty input, auto-confirm comparison), `word_challenge_*` (single-word output, pool membership, label/instruction passthrough), and `build_prompt_challenge_*` smoke tests across all basic actions
- The auto-confirm branch inside `handle_action_prompt_input` requires a Bevy `App`/world to drive and is left for the longer-term ECS test pass
- Target: `src/systems/interaction.rs`

**P1-B: Unit tests for NPC collision helper** ✅
- Extracted pure helper `resolve_aabb_push(entity_pos, entity_half, collider_pos, collider_half) -> Option<Vec2>` from `npc_collisions` in `src/systems/npc.rs`
- Unit tests cover non-overlap (returns `None`), horizontal-axis push (smaller overlap on x), vertical-axis push (smaller overlap on y), corner ties, and edge contact (zero overlap)
- Target: `src/systems/npc.rs`

---

### P2 - Visual polish: typing overlay entrance animation

Make the overlay feel snappier with a short fade-in and scale-up on appear.

**P2-A: Fade-in tween on overlay background** ✅
- Added `TypingOverlayFade` component (alpha: f32, TARGET_ALPHA = 0.82) to `src/components.rs`
- Overlay spawns with alpha 0 and `Visibility::Hidden`; `update_typing_overlay` animates alpha at 10 units/sec toward 0.82 each frame it is active
- On deactivate: alpha snaps to 0, `BackgroundColor` cleared, `Visibility::Hidden` set immediately
- Target: `src/components.rs`, `src/setup.rs`, `src/systems/hud.rs`

**P2-B: Scale-up tween on word row** ✅
- Added `TypingWordRow` marker and `TypingWordRowScale { scale }` component (`START_SCALE = 0.85`, `TARGET_SCALE = 1.0`, `RATE_PER_SEC = 1.25`) in `src/components.rs`
- Word row Node spawns with `Transform::from_scale(0.85)` and the new components in `src/setup.rs`
- New `update_typing_word_row_scale` system in `src/systems/hud.rs` lerps scale toward target while the prompt is active and snaps back when inactive (gives a ~120 ms entrance ease)
- Pure helper `next_word_row_scale(current, dt, active)` extracted and unit-tested (4 tests)
- Target: `src/components.rs`, `src/setup.rs`, `src/systems/hud.rs`, `src/main.rs`

---

### P3 - Audio: core sound effects

The `assets/audio/` folder is present but empty. Even minimal SFX dramatically improves feel.

**P3-A: Keypress sound** ✅
- `SfxKind::KeyPress` added; fires inside `handle_action_prompt_input` when buffer grows
- Asset: `assets/audio/key_press.ogg` (place a short click clip here)
- Target: `src/systems/interaction.rs`, `src/audio.rs`

**P3-B: Confirm sound** ✅
- `SfxKind::Confirm` added; fires on auto-confirm and Enter-confirm paths
- Asset: `assets/audio/confirm.ogg`
- Target: same as P3-A

**P3-C: Fail / wrong key sound** ✅
- `SfxKind::Fail` added; fires on failed Enter attempt (wrong word) and on retry exhaustion
- Asset: `assets/audio/fail.ogg`
- Note: all three degrade gracefully if asset files are absent (existing pattern)
- Target: `src/systems/interaction.rs`

---

### P4 - Gameplay depth

**P4-A: Job promotion event** ✅
- `WorkStreak::promotion_notified: u8` bitmask added in `src/resources.rs` (bit 0 = Senior, bit 1 = Executive)
- Senior promotion fires once at `career >= 2.5`, Executive at `career >= 5.0`, both gated by the bitmask so they only trigger one time each
- Promotion event surfaces a notification line and is logged through the narrative pipeline
- Field is persisted in `SaveData` (`save.rs`) so promotions survive across saves
- Target: `src/systems/interaction.rs` (work arm), `src/systems/narrative.rs`, `src/resources.rs`, `src/save.rs`

**P4-B: NPC hangout activity** ✅
- New `ActionKind::Hangout` interactable near an NPC when friendship >= 3
- Press **[H]** near an NPC to hang out (requires friendship >= 3); picks a word from `HANGOUT_WORDS` as challenge
- Grants +0.5 friendship, +25 happiness, -10 stress, -8 energy, +0.2 social skill XP
- NPC prompt now shows `[H] Hangout` hint once friendship level reaches 3
- Target: `src/components.rs`, `src/systems/interaction.rs`, `src/systems/npc.rs`

**P4-C: Apartment furnishing buffs** ✅
- New `Furnishings` component (desk, bed, kitchen flags) on the player entity
- Purchased at the Bank with F1/F2/F3 key shortcuts (requires apartment access, paid from savings)
  - F1 Desk $60: +15% skill XP on study/work
  - F2 Comfy Bed $80: +10 energy on each sleep
  - F3 Kitchen $100: -10 extra hunger reduction on each meal
- Persisted in save data; bank info message hints `[F1]Desk$60 [F2]Bed$80 [F3]Kitchen$100`
- Target: `src/components.rs`, `src/systems/interaction.rs`, `src/save.rs`, `src/setup.rs`

**P4-D: Visible skill tree panel** ✅
- New `SkillPanel` marker component plus per-skill bar markers (`SkillCookingBar`, `SkillCareerBar`, `SkillFitnessBar`, `SkillSocialBar`) in `src/components.rs`
- `spawn_skill_panel` builds a hidden bottom-right panel in `src/setup.rs`
- `toggle_skill_panel` (Tab key) and `update_skill_panel` (live bar widths and tier labels) added in `src/systems/hud.rs` and registered in `src/main.rs`
- Reads existing `Skills` component data; no new save fields
- Target: `src/setup.rs`, `src/systems/hud.rs`, `src/components.rs`, `src/main.rs`

---

### P5 - Fresh audit pass (Iteration 14) ✅

- Scanned all new systems added in P1-P4 + P3: Furnishings, Hangout, Promotion, TypingOverlayFade, SkillPanel, SFX variants
- Found and fixed T-04 violation: `sample_save()` missing four new `SaveData` fields (`promotion_notified`, `furnishing_*`)
- No new findings opened; no `.unwrap()` outside tests; no new functions >150 lines
- Audit logged in `audits/AUDIT-ITERATIONS-11-15.md` as Iteration 14

---

### Longer-term (post-P5)

These require significant architecture work and are not blocked by any of the above.

| Item | Description |
|---|---|
| Wasm deployment issue | Pre-existing "Startup error: Uncaught RuntimeError: unreachable" on deployed site. Not caused by P2-B, M-02, or any local changes (171 local tests pass, clean clippy). Deploy infrastructure issue - needs browser console diagnostics. |
| M-03 | Introduce a `PlayerAction` event abstraction to decouple raw keyboard input from game logic |
| M-04 | Restructure `SaveData` to support a `Vec<PlayerSave>` for per-player persistence |
| Art pass | Replace colored rectangles with sprite sheets for characters, buildings, and props |

### Mobile roadmap (M1-M5)

Mobile support work has started.

| Item | Description | Status |
|---|---|---|
| M1 | Touch input foundation for mobile web (virtual controls mapped to existing actions) | In progress (M1-A and M1-B complete) |
| M2 | Responsive HUD and layout tuning for phone and tablet viewports | Planned |
| M3 | Touch-first interaction UX for number/action selection flows | Planned |
| M4 | Mobile web performance and compatibility hardening | Planned |
| M5 | Mobile QA matrix and rollout criteria | Planned |

M1-A implemented in `index.html`:
- coarse-pointer mobile control overlay (virtual D-pad + sprint + action buttons)
- touch buttons emit keyboard-equivalent events so existing Bevy input systems keep working
- visibility-change safety releases held movement keys to avoid stuck movement

M1-B implemented in `index.html`:
- mobile on-screen typing keyboard (`ABC` toggle) with letters A-Z, Backspace, Enter, and Esc
- typing keys emit keyboard-equivalent events to reuse existing Bevy prompt input logic

**M-02: Replace `get_single()` calls with iterator-based access** ✅
- All 20 `get_single_mut()` sites in `src/systems/hud.rs` and `src/systems/visual.rs` migrated to `iter_mut().next()` (the singleton-friendly equivalent)
- Pattern: `let Ok(mut t) = q.get_single_mut() else { return; };` becomes `let Some(mut t) = q.iter_mut().next() else { return; };` (and the same for `if let` guards)
- Forward-compatible with Bevy's deprecation of the `get_single*` family and tolerates 2+ matching entities (no silent error masking when extra players or duplicate UI nodes are added)
- Verified: 171 tests passing, clean strict clippy

**Tutorial: First-run overlay** ✅
- `TUTORIAL_STEPS` and `TutorialState { step }` resource in `src/resources.rs` drive a 6-slide overlay (Welcome, Stats, Working, Eating/Sleeping, Socialising, Key Bindings)
- `start_tutorial_if_new_game` in `src/save.rs` flips `step` to 1 only when `GameStartKind == NewGame` so resumes never replay the tutorial
- `update_tutorial` in `src/systems/hud.rs` shows/advances slides on Space/Enter and dismisses on Esc
- Overlay node + body/hint text spawned by `setup` in `src/setup.rs`; hidden by default and shown by `update_tutorial` when `TutorialState::is_active()`

---

## Contributing

The project uses standard Rust tooling. Before submitting:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

No contribution guidelines are formalized yet. Open an issue before starting significant feature work.
