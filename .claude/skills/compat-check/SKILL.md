---
name: compat-check
description: Run the backward-compatibility gate for RustGame — build, unit tests, golden scenarios, and the golden-master campaign — then classify any failure as regression vs. declared behavior change by cross-checking the diff and CHANGELOG. Use after finishing a phase, before committing, or whenever the user asks to verify the game still behaves like before.
---

# Compat Check

You are verifying that the current working tree preserves the game's established behavior.
The contract: **every behavior change must be declared in CHANGELOG.md; anything the test
ladder catches that is not declared is a regression.**

## Procedure

Run the ladder in order; later steps only make sense if earlier ones pass.

1. **Build + warnings**
   ```bash
   cargo check 2>&1 | tail -20
   ```
   Compare the warning set against the CHANGELOG's declared expectations (currently: zero
   warnings — the lib split made `pub` scaffolding count as public API, clearing the old
   Phase 3 dead-code warnings). Any warning is a finding.

2. **Unit tests + golden scenarios + golden master**
   ```bash
   cargo test 2>&1 | tail -30
   ```
   This runs `src/**` unit tests, all of `tests/*.rs`, and the golden campaign. The campaign
   takes ~30–60s; that is normal. If compilation of tests fails, that is itself a finding
   (the lib API changed under the tests).

3. **On any failure, classify before touching anything**
   - Read the failing assertion and its message — scenario tests name the mechanic and the
     expected tuning value; the campaign names the first diverging frame and field.
   - Read the working diff (`git diff HEAD` and, if relevant, recent commits) and the
     `[Unreleased]` section of `CHANGELOG.md`.
   - Classify each failure:
     - **REGRESSION** — behavior changed and no CHANGELOG entry declares it. Report it with
       the failing test, the observed vs. expected values, and the most likely culprit
       system/file. Do NOT update baselines or assertions.
     - **DECLARED CHANGE** — the CHANGELOG explicitly describes this behavior change. Update
       the affected scenario assertions and/or regenerate the golden baseline:
       ```bash
       UPDATE_GOLDEN=1 cargo test --test golden_campaign
       ```
       State in your report exactly which baseline/assertions you updated and which
       CHANGELOG entry justifies each.
     - **NONDETERMINISM** — `campaign_is_reproducible_within_a_build` fails. Never
       regenerate around this; find the source (thread_rng in gameplay, unordered RunRng
       consumers, iteration-order dependence) and report it as a defect.

4. **Report** (final message):
   - Verdict line first: `PASS`, `PASS with declared changes (baseline updated)`, or
     `FAIL: N regressions`.
   - Then per finding: test name, one-sentence defect statement, evidence (observed vs.
     expected), suspected file:line.
   - Note anything the ladder cannot see yet (e.g. UI rendering is only verifiable on the
     Windows build — flag if the diff touched `src/ui/` or `presentation.rs`).

## Context you may need

- `docs/testing.md` — how the harness, scenarios, and baseline procedure work.
- `docs/architecture-plan.md` — the phase plan; §7 lists what each phase is allowed to change;
  **§8.5 is the tech-debt register** — check it before reporting a "finding" that is already a
  known, deliberately deferred item (report those as "known debt, tracked" instead).
- Baselines live in `tests/golden/`; their git history is the audit trail of declared
  behavior changes.
