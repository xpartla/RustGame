# Testing Infrastructure

_Established 2026-07-05 (testing-infrastructure stages 0–2), after Phase 2 of the migration
plan. This document explains how the game is tested headlessly, how the golden baselines
work, and how to trace a failure back to its cause._

---

## Why headless

The game cannot open a window under WSL (wgpu falls back to GL and panics before `Startup`),
and automated tests must not depend on a GPU anywhere. The gameplay simulation therefore runs
**without a renderer**:

- `GameLogicPlugin` (src/game/plugin.rs) — the complete simulation: movement, combat, abilities,
  talents, progression, enemies, world, pickups. No render/window types.
- `PresentationPlugin` (src/game/presentation.rs) — camera, UI, meshes/materials, map rendering,
  debug gizmos. Registered only by the windowed `GamePlugin`.

Logic spawns carry pure data (e.g. `EnemyAppearance`); presentation systems react to
`Added<T>` and attach `Transform` + `Mesh2d` + materials. The windowed game behaves exactly
as before the split.

## The sim harness (`src/sim/`)

`Sim` builds the full game app on `MinimalPlugins + StatesPlugin + AssetPlugin`:

- **Deterministic time** — `TimeUpdateStrategy::ManualDuration`: every `Sim::step` advances
  exactly `SIM_DT` (1/60 s).
- **Deterministic RNG** — the seed you pass becomes `RunRng` before the game plugins build.
- **Deterministic scheduling** — Startup/Update/PostUpdate run single-threaded.
- **Scripted input** — `ButtonInput` resources are initialized manually; `press_key` /
  `tap_key` / `press_mouse` drive the real input systems. Edge flags are cleared per frame
  by the harness, mirroring the real input pipeline.
- `Sim::new_arena(seed)` is the standard fixture: ambient spawners paused, map replaced by an
  empty bordered arena, all RON assets loaded.

Known nondeterminism that remains (by design, out of scenario scope):

- The **ambient enemy/pickup spawners** roll `rand::thread_rng` (spawn angle, archetype pick,
  pickup ring). Scenarios pause them and spawn actors explicitly. Enemy **death drops** were
  switched to `RunRng` because kills occur inside deterministic scenarios.
- In the **windowed build**, `Update` runs multi-threaded; ambiguous system pairs may
  interleave differently than in the sim. All gameplay-relevant orderings are pinned
  explicitly (CombatSet chain, `init_level_flow.after(generate_map)`,
  `refill_offer.after(install_acquired_talent)`).

## Test layers

| Layer | Where | What it locks in |
|---|---|---|
| Unit tests | `src/**` `#[cfg(test)]` | RON schemas parse, modifier-stack math, cone geometry, offer eligibility, band-pool flow |
| Golden scenarios | `tests/*.rs` | End-to-end behavior of one mechanic per test: movement/collision, Death Strike damage/leech/cooldown, contact damage cadence, XP → unlock → talent-picker round-trip, uniqueness filtering, pickups, map determinism |
| Golden master | `tests/golden_campaign.rs` | A 30-second scripted-bot campaign; a per-second trace of hp/level/xp/enemies/abilities/talents/position must match `tests/golden/campaign_baseline.ron` exactly |

Run everything: `cargo test`. Scenarios assert tuning values from the RON assets and
`archetypes()` — **changing tuning intentionally will fail tests; update the affected
assertions in the same change and say so in the CHANGELOG.**

## Golden baseline procedure (back-tracing)

The campaign baseline is the regression tripwire with the widest net. When
`campaign_matches_golden_baseline` fails:

1. The assertion names the first diverging frame/second and prints both snapshots — the
   diverging field (hp, level, enemies, …) narrows the subsystem immediately.
2. Cross-check the working diff and `CHANGELOG.md`: is the divergence explained by a declared,
   intentional behavior change?
   - **Not declared → treat as a regression.** Bisect with the focused scenario tests.
   - **Declared → regenerate deliberately:**
     `UPDATE_GOLDEN=1 cargo test --test golden_campaign`
     and commit the new baseline **in the same commit** as the change that explains it, so
     `git log tests/golden/` is a full audit trail of behavior changes.
3. `campaign_is_reproducible_within_a_build` failing instead means new nondeterminism leaked
   into the simulation (thread_rng in a gameplay system, an unordered `RunRng` consumer,
   iteration-order dependence). Fix the source; do not regenerate the baseline around it.

Baselines are pinned by `Cargo.lock` (rand/bevy algorithm stability) and generated on this
machine; a toolchain or dependency bump that shifts float behavior is itself a
baseline-regeneration event — declare it in the CHANGELOG like any other behavior change.

## Adding scenarios (definition of done per phase)

Every phase from Phase 3 onward should land with golden scenarios for its mechanic, e.g.:

- Phase 3: "frost-tagged damage removes blaze", "bleed ticks N damage over M seconds".
- Phase 4: "stance swap remaps LMB", "second class basic attack fires through input slots".
- Phase 5: "EnemyDef RON spawns an enemy with the declared stats".
- Phase 6: "D&D zone doubles Blood Boil range inside it".
- Phase 7: "act graph is seed-deterministic", "objective completion advances the node".

Keep each scenario one mechanic; put cross-system drift detection in the campaign baseline.

## The compat agent

`/compat-check` (`.claude/skills/compat-check/SKILL.md`) runs the whole ladder — build,
warnings, unit tests, scenarios, golden master — then classifies failures as regression vs.
declared change by reading the diff and CHANGELOG, and reports with file/line references.
It is the on-demand backward-compatibility gate to run after each phase (or before a commit).

## Balance testing (later stages)

The same harness is the substrate for balance evaluation once enemies are data-driven with a
scaling model (Phase 5+) and encounters exist (Phase 7):

- an `arena` binary running hero × build × encounter × seed sweeps at high speed,
- `BotPolicy` implementations as the stand-in player (deterministic, comparable),
- JSONL metrics (clear time, DPS in/out, TTK per enemy type, level curve) consumed by a
  balance-analyst agent that ranks builds, flags outliers, and proposes RON tuning diffs.
