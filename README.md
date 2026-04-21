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
- full-screen typing overlay: one random word per action, per-character green/highlight/gray feedback, auto-confirms on completion
- expanded word banks (15-20 words per action category, 35+ categories) for high variety
- save and load with JSON persistence
- Menu, Playing, Paused, and Settings screens
- banking, loans, investments, housing, and transport upgrades
- NPC friendship, quests, narrative unlocks, and reputation systems
- pets, crisis events, seasonal festivals, and weather-driven visuals
- universal typed action prompts with seniority-based retries and subject-aware phrases before tasks resolve

The current baseline is verified with a successful build, a clean strict clippy run, and 150 passing tests.

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

Notes for the web build:
- game assets are served from the bundled assets directory
- settings and save progress live in browser localStorage instead of config.toml and save.json
- the canvas automatically fits the browser page

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
| Tests | 150 passing |
| Save and load | Implemented |
| Crisis system | Implemented |
| Seasonal festivals | Implemented |
| Settings screen | Implemented |
| Audio fallback | Implemented |
| Browser build | Live at rodmen07.github.io/new_game |
| Multiplayer support | Early groundwork only |

The project is currently a playable, feature-rich prototype with a clean verified Rust baseline and ongoing expansion work.

## Near-Term Roadmap

### In progress
- Continued refactor of player-specific state from global resources to entity components
- Deeper multiplayer-safe ECS cleanup and architecture follow-through

### Next content goals
- More NPC depth and relationship states
- Character archetypes and stronger replay loops
- Art pass, tutorial flow, and accessibility improvements

### Known follow-ups
- Multiplayer readiness is still architectural groundwork rather than a playable mode

---

## Contributing

The project uses standard Rust tooling. Before submitting:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

No contribution guidelines are formalized yet. Open an issue before starting significant feature work.
