# Audit Dimensions

This document defines all dimensions the audit process checks. Each dimension has
a category prefix, a checklist of what to scan, and severity guidelines.

---

## R - Readability / Maintainability

Scan for:
- [ ] Functions longer than ~100 lines
- [ ] Repeated code blocks (3+ occurrences)
- [ ] Deep nesting (4+ indent levels)
- [ ] Opaque variable names (`_m`, `x`, `tmp`)
- [ ] Missing section comments in long functions
- [ ] God-object structs (resource with 20+ fields - consider splitting)

Severity: functions >200 lines = High, >100 = Medium

---

## I - Idiomatic Rust

Scan for:
- [ ] `.unwrap()` / `.expect()` outside tests
- [ ] `.clone()` on `Copy` types
- [ ] Manual enum-to-integer conversions (should use From/TryFrom)
- [ ] Unnecessary `mut` bindings
- [ ] Intermediate `Vec` allocations that could be iterators
- [ ] String formatting that could use `write!` or `Display`

Severity: unwrap in system = High, style issues = Low

---

## T - Test Coverage

Scan for:
- [ ] Files with zero `#[test]` functions
- [ ] Public functions with no test path (even indirect)
- [ ] Pure helper functions that should have unit tests
- [ ] Boundary conditions not covered (0, max, overflow)
- [ ] Test ratio: target 1 test per 15-20 lines of logic

Metrics to track each iteration:
```
Test count: N
Public fn count: M
Ratio: N/M
Files with zero tests: [list]
```

Severity: zero-test file with >100 lines = High, missing boundary tests = Medium

---

## S - Safety / Reliability

Scan for:
- [ ] `unsafe` blocks (should be zero)
- [ ] Unvalidated external input (file reads, config parsing)
- [ ] Silent error swallowing (`let _ = ...` on Result)
- [ ] Panic paths in non-test code
- [ ] Resource exhaustion (unbounded Vec/HashMap growth)
- [ ] Save/load field sync (new fields in struct but not in sample_save)

Severity: panic path = High, silent error = Medium

---

## G - Gameplay Depth

Evaluate whether game systems create meaningful player choices and progression curves.

Scan for:
- [ ] Dead-end mechanics (systems the player has no reason to engage with)
- [ ] Dominant strategies (one approach is always optimal, making others pointless)
- [ ] Flat progression (no meaningful difference between early and late game)
- [ ] Missing feedback loops (player actions with no visible consequence)
- [ ] Unbalanced economy (money too easy or too hard to earn at any stage)
- [ ] Skill ceiling (is there room for experienced players to optimize?)
- [ ] Event variety (are daily events / crises / festivals varied enough?)
- [ ] NPC depth (do NPCs feel distinct beyond personality label?)

Evaluation method:
1. Trace a "first 10 days" playthrough - what choices does the player face?
2. Trace a "day 50+" playthrough - is the game still engaging?
3. Check for degenerate loops (e.g., spam one action, ignore everything else)
4. Verify each ActionKind has a reason to be used

Severity: dominant strategy that trivializes the game = High, missing variety = Medium

---

## Q - Quality of Life

Evaluate player experience friction points.

Scan for:
- [ ] Missing or unclear feedback (player does something, nothing visible happens)
- [ ] Information overload (HUD showing too much at once)
- [ ] Tedious repetition (player must repeat the same inputs many times)
- [ ] Missing hotkeys or shortcuts for common actions
- [ ] Unclear objectives (player doesn't know what to do next)
- [ ] Notification spam (too many messages overlapping)
- [ ] Missing visual/audio feedback for important state changes
- [ ] Confusing spatial layout (player gets lost or can't find buildings)
- [ ] Save/load friction (is auto-save reliable? manual save accessible?)
- [ ] Settings coverage (can player adjust things that matter to them?)
- [ ] Onboarding (does a new player understand the controls and goals?)

Evaluation method:
1. Read all notification strings - are they clear and actionable?
2. Check HUD layout - is critical info always visible?
3. Review keybindings - are they documented and intuitive?
4. Check for "information at the right time" - does the player learn mechanics gradually?

Severity: broken feedback = High, missing polish = Low

---

## M - Multiplayer Readiness

Evaluate architecture readiness for networked multiplayer.

Scan for:
- [ ] Global mutable state that assumes single player
- [ ] Player identity: is the player entity distinguishable from other entities?
- [ ] Time synchronization: is game time driven by a central clock or per-client?
- [ ] Input handling: are inputs tied to a specific player or assumed global?
- [ ] Save format: can it represent multiple players?
- [ ] Determinism: are game outcomes reproducible given the same inputs?
- [ ] Resource ownership: can resources be scoped per-player?
- [ ] Action validation: are actions validated server-side or trust-the-client?

This dimension is informational until multiplayer development begins.
Findings here are tagged with severity "Planning" rather than the usual scale.

---

## P - Performance

Scan for:
- [ ] O(n^2) or worse algorithms
- [ ] Per-frame allocations (Vec::new() in systems that run every tick)
- [ ] Unnecessary entity queries (querying all entities when only one is needed)
- [ ] Large struct copies where references would suffice
- [ ] Missing `With`/`Without` filters on queries

Severity: per-frame allocation in hot path = Medium, theoretical concern = Low

---

## How to Use This Document

During Phase 1 (Gather Scope), the agent scans each dimension's checklist against
the codebase. New findings are recorded in `AUDIT-SUMMARY.md` with the appropriate category
prefix (R-XX, I-XX, T-XX, S-XX, G-XX, Q-XX, M-XX, P-XX).

Priority order for fixing:
1. S (Safety) - fix immediately
2. T (Coverage) - fix before adding features
3. R (Readability) - fix to unblock future work
4. G (Gameplay) - address for game quality
5. Q (Quality of Life) - address for player experience
6. I (Idiomatic) - address when touching related code
7. P (Performance) - address when measured bottleneck exists
8. M (Multiplayer) - informational until multiplayer work begins
