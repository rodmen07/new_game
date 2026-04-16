# Everyday Life Simulator

A real-time 2D life management sim written in Rust with Bevy. You manage a single character's day-to-day existence — balancing energy, hunger, finances, career progression, social relationships, and housing — across an accelerated 24-hour cycle in a small procedurally-driven urban world.

---

## Overview

Genre: life sim / idle RPG  
Perspective: top-down 2D  
Time scale: 60× (1 real second = 1 in-game minute)

The core loop is daily survival and long-term progression. Each day you work, eat, sleep, interact with NPCs, and pursue goals. Stats decay continuously; neglect any of them long enough and cascading penalties kick in (burnout, malnourishment, eviction). Progress unlocks better housing, career ranks, passive income streams, and one of 15 milestone achievements. The game grades your life daily on an F–S scale.

The project is currently a feature-complete prototype with no save system, no audio, and a single world layout.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust (Edition 2024) |
| Engine | [Bevy](https://bevyengine.org/) 0.15 |
| Architecture | Entity-Component-System (ECS) |
| Rendering | Bevy 2D sprite pipeline (no external renderer) |
| RNG | Linear congruential generator (seeded per day) |
| Dependencies | Bevy only (single entry in `Cargo.toml`) |

Bevy's ECS drives the entire game: all game state lives in `Resource` structs; all logic lives in `System` functions scheduled by Bevy's scheduler. Custom `SystemParam` groups work around Bevy's 16-parameter system limit.

---

## Building & Running

### Prerequisites

- Rust stable toolchain (1.80+): https://rustup.rs
- On Linux: `libudev-dev`, `libasound2-dev`, `libxkbcommon-dev` (Bevy window/audio deps)
- On Windows/macOS: no extra system dependencies

### Build

```bash
# Debug (fast compile, slow runtime)
cargo run

# Release (optimized — recommended for gameplay)
cargo run --release
```

The first build downloads and compiles Bevy's dependency tree (~200 crates). Expect 2–5 minutes on first run; subsequent builds are incremental.

### Optional: faster debug builds

Add to `.cargo/config.toml` at the project root:

```toml
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

---

## Controls

| Key | Action |
|---|---|
| `W A S D` / Arrow keys | Move |
| `Shift` (hold) | Sprint (1.75× speed, drains energy) |
| `Space` | Dash (burst 620 px/s for 0.18 s, 1.2 s cooldown) |
| `E` | Interact with nearest object / NPC |
| `G` | Gift (when adjacent to a max-friendship NPC) |
| `1`–`8` | Bank shortcuts (deposit, withdraw, loan, invest, etc.) |
| Scroll wheel | Camera zoom (0.35×–2.5×) |

Interaction radius: 58 px. A prompt at the bottom of the screen shows the available action when in range.

---

## Architecture

```
src/
├── main.rs          # App entry point, system scheduling, resource init
├── components.rs    # All ECS component definitions
├── constants.rs     # Numeric tuning constants (speeds, radii, timings)
├── resources.rs     # All ECS resources (PlayerStats, Skills, Hobbies,
│                    #   Housing, Transport, Pet, Investment, Reputation, …)
├── setup.rs         # World spawn: geometry, interactables, NPCs, HUD
└── systems/
    ├── mod.rs
    ├── player.rs    # Movement, sprint, dash, camera, body animation
    ├── collision.rs # AABB resolution, least-penetration ordering
    ├── npc.rs       # Zone wandering, friendship, walk cycle, labels
    ├── interaction.rs # Proximity detection, action dispatch, cooldowns
    ├── stats.rs     # Continuous stat decay, condition triggers
    ├── time.rs      # Game clock, day transitions, daily events
    ├── goals.rs     # 18-day goal cycle, milestone evaluation
    ├── hud.rs       # All UI text/bar updates
    └── visual.rs    # Day/night overlay, highlights, dash particles
```

### Key design decisions

- **Resource-driven state**: All mutable game state (`PlayerStats`, `Skills`, `GameClock`, etc.) is stored in Bevy `Resource`s, not on entities. This keeps queries simple and avoids multi-entity synchronization.
- **LCG pseudo-RNG**: NPC wandering and daily event selection use a seeded linear congruential generator (`seed = day_number`), giving deterministic but varied daily content without pulling in an RNG crate.
- **SystemParam groups**: Custom parameter structs (`HudExtras`, `InteractExtras`, `DayExtras`) bundle related queries/resources to stay under Bevy's 16-argument system limit.
- **Layered Z-ordering**: All 2D depth is explicit (0–50 range); no scene graph.

---

## Current Feature Set

### Player & Movement
- Acceleration/friction model (1400/900 px/s²), speed capped by health and energy thresholds
- Dash with trail particles; dash cancelled and penalized on wall collision
- Sprint stamina drain; movement penalties below 30 HP or 20 energy
- Transport modifier: Walk 1.0×, Bike 1.1×, Car 1.3× (speed and work pay)

### Stats (all 0–100)
- **Energy** — drains 0.55/s; weather-modified (Stormy 1.5×); recoverable via sleep, coffee, items
- **Hunger** — increases 0.8/s; causes health damage above 80; cured by eating
- **Happiness** — degrades from hunger/exhaustion; boosted by social actions, hobbies, outdoor weather
- **Health** — damaged by severe hunger, sleep deprivation, high stress; blocks work below threshold
- **Stress** — increased by work (+8), loan debt, random events; reduced by meditation (−25), relaxation (−12)
- **Sleep Debt** — accumulates 8 h/day; caps max energy at 80 (8–16 h debt) or 60 (>16 h)

### Conditions
- **Burnout**: triggers after 3 consecutive high-stress days; −30% work pay
- **Malnourished**: triggers after 3 consecutive high-hunger days; health degrades 1.5× faster

### Economy
- Cash, savings (5%/day interest), loan (8%/day interest, hard cap $300 before repay required)
- Daily rent by housing tier; 3-day unpaid rent → eviction
- Investments: low-risk (4%/day) and medium-risk (10%/day) with seeded daily variance
- Work pay formula: `base × career_bonus × mood × time_of_day × weekend × stress × loan_penalty × burnout × transport`

### Progression
- **Career** (0–5): Junior ($30/session) → Senior ($45) → Executive ($70), +12% per level
- **Skills**: Cooking, Fitness, Social (0–5 each); bonuses applied to relevant actions
- **Hobbies**: Painting ($6/day passive), Gaming ($4/day), Music ($8/day) — unlocks at level 3+
- **Housing**: Apartment → Condo ($200) → Penthouse ($500); higher tiers improve sleep recovery and morning bonuses
- **Transport**: upgradeable via savings; affects movement speed and work pay multiplier

### World & Social
- 7 named zones: Home, Office, Park, Store, Bank, Library, Garage
- 21 action types across 13+ interactable objects
- 3 NPCs (Alex, Sam, Mia) with zone-based schedules, LCG wandering, friendship tracking (0–5)
- Friendship decay (−0.15/day); gifting unlocked at max friendship
- 18-day rotating goal cycle with seasonal variants; 15 unlockable milestones
- Life Rating calculated daily: weighted composite of all stats → grade F through S

### Time & Environment
- 24-hour accelerated clock; time-of-day bonuses (Early Bird, Late Night)
- 4 weather types (Sunny/Cloudy/Rainy/Stormy) generated procedurally per day
- 4 seasons (30-day cycles) with per-season stat multipliers and seasonal goals
- 14 randomized daily events (payday, bills, found money, noise complaints, etc.)

### Rendering & HUD
- Composite humanoid sprites (7 layered rectangles per character), walk cycle animation
- Day/night ambient color overlay (dawn orange → clear day → dusk gold → night blue)
- Two HUD panels: left (stats, skills, inventory) and right (goals, conditions, progression)
- Scrolling notification banner, context-sensitive bottom prompt
- Camera: smooth lerp follow, scroll-to-zoom

---

## Development Roadmap

Current state: functional prototype, ~1,600 lines, no persistence, no audio.

### Q1 — Foundation Hardening & Core UX (Months 1–3)

| # | Milestone |
|---|---|
| 1 | **Save/load system** — serialize all `Resource`s to disk with `serde` + RON/JSON; auto-save on day end |
| 2 | **Bevy states** — `MenuState` / `PlayState` / `PausedState`; main menu and pause screen |
| 3 | **Audio** — integrate `bevy_kira_audio`; ambient loop, per-action SFX, day-transition chime |
| 4 | **Settings file** — resolution, volume, key remapping persisted to `config.toml` |
| 5 | **Bug pass** — collision tunneling at high speed, HUD text overflow, energy edge cases |
| 6 | **CI** — GitHub Actions pipeline: `cargo fmt`, `cargo clippy`, `cargo test` on every PR |

### Q2 — Content Expansion & World Building (Months 4–6)

| # | Milestone |
|---|---|
| 1 | **Event pool expansion** — 14 → 40+ daily events, weighted by season, reputation, and active conditions |
| 2 | **3 additional NPCs** (total 6) with distinct personalities; personality affects chat gain and gift outcome |
| 3 | **NPC daily schedules** — time-of-day zone transitions (office 9–17, park evenings, home at night) |
| 4 | **Second district** — new map area with gym, café, and clinic interactables |
| 5 | **Narrative arcs** — short milestone-triggered storylines (e.g., promotion arc at Career 3, housing arc at Penthouse) |
| 6 | **Item system expansion** — stackable inventory, item expiry, use-on-NPC actions |

### Q3 — Gameplay Depth & Replayability (Months 7–9)

| # | Milestone |
|---|---|
| 1 | **Difficulty modes** — Easy / Normal / Hard adjusting drain rates, interest, rent, and event severity |
| 2 | **Character archetypes** — starting selection (Student / Professional / Artist) with different stat baselines |
| 3 | **Crisis events** — layoffs, medical emergencies, market crashes; require multi-day recovery |
| 4 | **Relationship depth** — acquaintance → friend → partner states per NPC; unlockable NPC side quests |
| 5 | **New game+** — prestige mode after S-grade sustained for 30 days; carry-over stat bonuses |
| 6 | **Tilemap migration** — replace colored-rect world geometry with `bevy_ecs_tilemap` sprite tiles |
| 7 | **Accessibility** — colorblind palettes, UI scale setting, persistent event log panel |

### Q4 — Polish, Release Prep & Distribution (Months 10–12)

| # | Milestone |
|---|---|
| 1 | **Sprite art pass** — replace composite-rect characters with `TextureAtlas` sprite sheets; frame animation |
| 2 | **Tutorial** — guided first-day walkthrough covering core loops; skippable after completion |
| 3 | **Localization scaffolding** — externalize all UI strings; ship `en-US` baseline, support additional locales |
| 4 | **Achievements backend** — local file mirroring the milestone system; Steam API integration points |
| 5 | **Performance profiling** — Tracy integration, entity batching, reduce draw calls |
| 6 | **WASM build** — target `wasm32-unknown-unknown`, bundle with `trunk`, test in browser |
| 7 | **itch.io release** — web build + Windows binary, store page, initial devlog post |

---

## Contributing

The project uses standard Rust tooling. Before submitting:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

No contribution guidelines are formalized yet. Open an issue before starting significant feature work.
