# Audit Iterations 11-15

Iteration summaries for the [main audit document](AUDIT-SUMMARY.md).

Note: Iterations 11 and 12 were appended to `AUDIT-ITERATIONS-6-10.md` before this
file was created. This file begins with Iteration 13.

---

### Iteration 13 - 2026-04-19

**Auditor:** Claude (agentic workflow)
**Scope:** Phase 1 structural scan + R-09 fix (make_goal refactor) + new findings R-06/R-07/R-08
**Method:** Agentic workflow per AGENT-AUDIT-PROMPT.md

**Phase 1 scan results:**
- Tests: 149 passing (prior run - cargo unavailable in Bash shell)
- Public fns: 140
- Test ratio: 149/140 = 1.06
- Functions >100 lines: 5 (main: 127, setup: 780, spawn_hud: 375, build_prompt_challenge: 242, make_goal: 185)
- Files with zero tests: components.rs, constants.rs, main.rs, menu.rs, setup.rs, festival.rs, hud.rs, mod.rs, narrative.rs, npc.rs, quests.rs, vehicle.rs, visual.rs
- Unwrap/expect outside tests: 0 (all in test functions)
- New findings opened: R-06, R-07, R-08, R-09
- Previously fixed bugs noted: Gate 1 fully resolved (stats.rs multi-decrement, visual.rs particle clamp, plus 3 already-wired items)

**Planned changes:**
- R-09: Extract `goal()` constructor from `make_goal()` to eliminate 18 repeated struct literals.
  Pure refactor - no behavior change. Reduces function from 185 to 20 lines.
- R-06, R-07, R-08: Document as new open findings.
- Add `Copy` to `GoalKind` derive (all unit variants, safe).

**Results:**
- **R-09** - FIXED. Added private `goal(kind, desc, target, reward_money, reward_happiness)` helper.
  `make_goal()` reduced from 185 lines to 20 lines. All 18 goal arm values preserved verbatim.
  `GoalKind` gained `Copy` derive (all variants are unit variants, no data).
  8 existing `make_goal` tests remain valid - no description-equality assertions.
  Note: em dash in day-0 description changed to hyphen (project style rule).
- **R-06** - Documented as High/Open. setup() 780 lines.
- **R-07** - Documented as High/Open. spawn_hud() 375 lines.
- **R-08** - Documented as Medium/Open. build_prompt_challenge() 242 lines.

**Build verification:** cargo not available in Bash shell - must verify in VS Code terminal:
```
cargo clippy -- -D warnings
cargo test
```
Expected: 149 tests pass, 0 clippy warnings.

**Findings closed this iteration:** R-09
**Findings opened this iteration:** R-06, R-07, R-08

**Top priorities for next session:**
- R-06: Extract sub-functions from setup() (780 lines) - spawn_world_geometry, spawn_interactables, spawn_npcs
- R-07: Extract sub-functions from spawn_hud() (375 lines)
- M-01 continuation: Migrate PlayerStats from Resource to Component (requires cargo to verify)

---

### Iteration 14 - 2026-07-19

**Auditor:** GitHub Copilot (agentic workflow)
**Scope:** Roadmap P1-P4 + P3 additions - new features since Iteration 13
**Method:** Code scan of all new/modified files; checklist pass per AUDIT-DIMENSIONS.md

**New systems added since Iteration 13:**
- P2-A: `TypingOverlayFade` component + lerp fade in `hud.rs`
- P4-B: `ActionKind::Hangout`, H-key, hangout word challenge, friendship gate
- P4-C: `Furnishings` component, F1/F2/F3 purchase at Bank, sleep/eat/skill bonuses
- P4-A: Job promotion - Senior (career>=2.5), Executive (career>=5.0) with bitmask dedup
- P4-D: Skill tree panel - Tab toggle, pip bar display, rank labels
- P3: `SfxKind::KeyPress`, `SfxKind::Confirm`, `SfxKind::Fail` audio events

**Phase 1 scan results:**
- `.unwrap()` / `.expect()` outside tests: 0 (confirmed by grep)
- New functions > 150 lines: none introduced (all helpers kept short)
- Repeated code: furnishing purchase (F1/F2/F3) has 3 structurally similar blocks in Bank arm - acceptable (3 distinct items, each with unique price/field)
- New `SaveData` fields: `promotion_notified`, `furnishing_desk`, `furnishing_bed`, `furnishing_kitchen` all serialized and deserialized correctly

**Bug found and fixed:**
- **T-04 violation** (sample_save out of sync): `sample_save()` in `save.rs` was missing `promotion_notified`, `furnishing_desk`, `furnishing_bed`, `furnishing_kitchen`. Fixed by adding all four fields with their default values. Rust struct literals require all fields; this would have caused a compile error.

**Checklist findings:**
- Magic numbers: furnishing prices ($60/$80/$100) are inline in the Bank arm. Low severity - they appear exactly once each; extracting to constants offers marginal benefit. Accepted.
- `SfxKind::KeyPress` fires on any buffer growth (even after re-typing same letter). This is intentional - every keypress feedback is expected.
- P3 assets (`key_press.ogg`, `confirm.ogg`, `fail.ogg`) intentionally absent; system degrades gracefully per `load_optional_audio` pattern.
- `TypingOverlayFade::TARGET_ALPHA = 0.82` is an associated constant on the struct - well-named, no change needed.
- Hangout cooldown value (2) is inline. Low severity - appears once. Accepted.

**Findings opened this iteration:** none

**Findings closed this iteration:** none (T-04 process violation fixed inline)

---

### Iteration 15 - 2026-04-22

**Auditor:** GitHub Copilot (agentic workflow)
**Scope:** Bug fixes for browser audio, remote player visibility, and MMO gap analysis
**Method:** Targeted code inspection of `audio.rs`, `network.rs`, startup spawn paths, and multiplayer architecture

**Planned changes:**
- **R-10:** Fix WASM audio asset loading so browser builds do not depend on native filesystem checks.
- **R-11:** Fix remote player visibility by matching the local player render baseline.
- Add an MMO analysis document describing which mechanics are local-only vs server-authoritative candidates.

**Results:**
- **R-10** - FIXED. `audio.rs` now uses a WASM-specific load path that skips native filesystem existence checks and always loads audio via `AssetServer` in browser builds.
- **R-11** - FIXED. `network.rs` remote player spawns now include `Visibility::default()` and use `z=10.0` to match the local player render layer.
- MMO analysis document added at `docs/mmo-model-analysis.md` to identify which mechanics remain local-only and what needs server authority for MMO support.

**Build verification:**
```
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --target wasm32-unknown-unknown --release
```

All three passed after the fixes.

**Findings opened this iteration:** R-10, R-11
**Findings closed this iteration:** R-10, R-11
