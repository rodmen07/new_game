# Agentic Audit Process - Everyday Life Simulator

This document defines a repeatable, self-contained audit workflow that an AI agent
can execute autonomously. Each run produces one iteration entry.

---

## Pre-Conditions

- Workspace: `d:\Projects\new_game`
- Build tool: `cargo build`, `cargo test`, `cargo clippy -- -D warnings`
- Docs: `audits/AUDIT-SUMMARY.md` (findings), `audits/AUDIT-ITERATIONS-*.md` (logs)
- Platform: Windows, Rust Edition 2024, Bevy 0.15

---

## Phase 1: Gather Scope (read-only)

1. Read `audits/AUDIT-SUMMARY.md` - identify all findings and their statuses
2. Read the latest `audits/AUDIT-ITERATIONS-*.md` - find the most recent iteration
3. Run automated checks:
   ```bash
   cargo clippy -- -D warnings 2>&1
   cargo test 2>&1
   ```
4. Run structural scans:
   ```bash
   # Functions over 100 lines
   python audits/scan_long_functions.py src/

   # Files with zero tests
   for f in src/**/*.rs; do grep -q "#\[test\]" "$f" || echo "NO TESTS: $f"; done

   # Unwrap/expect outside tests
   grep -rn "\.unwrap()\|\.expect(" src/ | grep -v "#\[test\]" | grep -v "mod tests"

   # Public function count vs test count
   echo "Public fns: $(grep -rn 'pub fn ' src/ | wc -l)"
   echo "Tests: $(grep -rn '#\[test\]' src/ | wc -l)"
   ```
5. Read `audits/AUDIT-DIMENSIONS.md` for the full checklist of audit dimensions
6. Produce a scope summary: what's open, what's new, what's the priority order

---

## Phase 2: Create Plan + Claim Work

1. Decide which findings to address (max 3-4 per iteration to keep diffs reviewable)
2. Update `AUDIT-SUMMARY.md`: set chosen findings to "In Progress (Iteration N)"
3. Create the iteration entry in the appropriate `AUDIT-ITERATIONS-*.md` with:
   - Planned changes (specific, measurable)
   - Empty Results section marked "_(to be filled after work completes)_"

---

## Phase 3: Execute

For each planned change:
1. Read the target file(s)
2. Make the code change
3. Run `cargo build` - fix any errors before proceeding
4. Run `cargo test` - all tests must pass before moving to the next change
5. If a change breaks tests, diagnose and fix before continuing

Rules:
- No behavior changes unless explicitly planned
- Extract, don't rewrite
- Prefer pure helper functions (testable without Bevy World)
- Follow existing code style (no new dependencies without justification)
- Never use em dashes; use hyphens instead

---

## Phase 4: Verify

1. `cargo clippy -- -D warnings` - zero warnings
2. `cargo test` - all tests pass, count matches or exceeds previous iteration
3. `cargo build` - clean build
4. Re-run structural scans from Phase 1 to confirm improvements

---

## Phase 5: Document Results

1. Fill in the Results section of the iteration entry
2. Update finding statuses in `AUDIT-SUMMARY.md` (Fixed / Accepted / still In Progress)
3. Update iteration count in the Iteration Log links
4. List top priorities for next session

---

## Phase 6: Self-Assessment

Before completing, answer:
- Did test count increase, stay the same, or decrease?
- Are there any new clippy warnings?
- Did any finding get worse?
- What should the next iteration focus on?

---

## Audit Dimensions

See `audits/AUDIT-DIMENSIONS.md` for the full matrix of what to check.
The agent should scan ALL dimensions each iteration, but only act on the
highest-priority open items.

---

## Iteration Naming

Format: `Iteration N - YYYY-MM-DD`
Iterations 1-5 go in `AUDIT-ITERATIONS-1-5.md`, 6-10 in `AUDIT-ITERATIONS-6-10.md`, etc.

---

## Emergency Stop Conditions

Do NOT proceed if:
- `cargo build` fails and you can't diagnose within 3 attempts
- Test count drops by more than 5
- You're unsure whether a change alters gameplay behavior
- A finding requires adding a new crate dependency (flag for human review)
