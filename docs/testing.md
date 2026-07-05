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
  explicitly (MovementSet → CombatSet/StatusSet chain, `init_level_flow.after(generate_map)`,
  `refill_offer.after(install_acquired_talent)`).

Two scheduling guarantees added by Phase 3.1 (both are contracts the tests rely on):

- **Movement is pinned** (`MovementSet::Intent → MovementSet::Integrate → CombatSet::Damage`),
  so adding Update systems in later phases does not reshuffle position math and nudge the
  golden master's px/py. If a future px/py-only baseline drift appears anyway, something
  bypassed the sets — treat it as a finding, not an automatic regen.
- **Overlay freeze preserves in-flight combat events.** `DamageEvent`, `HealEvent`,
  `ApplyStatusEvent`, `RemoveStatusEvent` are registered via `add_gameplay_event`
  (core/events.rs): their buffers advance only during InRun frames, so an event written the
  frame the TalentPicker (or any overlay) opens resolves on the first frame after resume
  instead of silently expiring. Terminal states (GameOver, Menu) clear them. Input-intent
  events still expire normally. When adding a NEW gameplay event, choose deliberately:
  combat-resolution → `add_gameplay_event`, input-intent or same-frame-consumed → `add_event`.

## Test layers

| Layer | Where | What it locks in |
|---|---|---|
| Unit tests | `src/**` `#[cfg(test)]` | RON schemas parse (abilities, talents, statuses), modifier-stack math, cone geometry, offer eligibility, band-pool flow |
| Golden scenarios | `tests/*.rs` | End-to-end behavior of one mechanic per test: movement/collision, Death Strike damage/leech/cooldown, contact damage cadence, XP → unlock → talent-picker round-trip, uniqueness filtering, pickups, map determinism; status lifecycle (DoT cadence, stacking rules, CC, cross-element cancellation, kill credit, orphan reaping), projectiles (travel-then-hit, status-on-impact, pierce), auto-cast + aim gate, overlay freeze semantics (`tests/freeze.rs`) |
| Golden master | `tests/golden_campaign.rs` | A 30-second scripted-bot campaign; a per-second trace of hp/level/xp/enemies/abilities/talents/**statuses**/position must match `tests/golden/campaign_baseline.ron` exactly. Since Phase 3 the bot also casts Frostbolt (projectiles + frostbite) and Blood Boil auto-casts, so the master covers status/projectile drift |

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

- Phase 3 (done): `tests/status.rs`, `tests/projectile.rs`, `tests/autocast.rs` — bleed cadence,
  frostbite slow/amp, root/stun, fire↔frost cancellation, DoT kill credit, projectile
  travel-then-hit, status-on-impact, Blood Boil auto-cast, the per-behavior aim gate.
- Phase 4 (done): `tests/hero_stance.rs` — "stance swap remaps LMB", "second class basic attack
  fires through input slots", plus DK LMB regression, swap-effect applied (Ice Barrier / Boots of
  Fire), non-stance Q no-op, and the debug (M) hero swap. Plus unit tests: `DefLibrary::get`,
  `HeroDef` RON parse (DK + Mage), and the pure `resolve_slot` (stance → InputSlot → AbilityId).
- Phase 5 (done): `tests/enemy.rs` — "EnemyDef RON spawns an enemy with the declared stats", enemy
  contact hits the player not other enemies (faction), player casts don't self-hit, the ranged caster
  stops-and-shoots, an enemy bolt passes through a Hostile to hit the player, scaling grows
  health+damage by depth, and a suppressed (stunned) caster can't cast. Plus unit tests: `EnemyDef`
  RON parse and `resolve_enemy_stats` scaling math. Contact cadence stays in `tests/combat.rs`.
- Phase 6 (done): `tests/zone.rs` — "D&D zone doubles Blood Boil range inside it" (the code-driven
  hook DoD), plus: a dropped zone spawns and expires, presence tracks enter/exit, Consecrated Ground
  DoT damages enemies inside but not outside and never the Friendly owner, D&D regen heals the owner
  inside only, AMZ destroys an enemy bolt entering it, a bolt emitted from inside the AMZ is not
  blocked, and a follow-anchor zone tracks its owner. Plus unit tests: zone RON parse
  (`dnd`/`tree_conduit`/`consecrated_ground`/`amz` + no-zone default) and the `blood_boil_dnd_range`
  hook doubling `radius` only inside D&D.
- Phase 7 (done): `tests/act_graph.rs` — "act graph is seed-deterministic", "graph is connected with
  one act boss" — and `tests/encounter.rs` — "objective completion advances the node", plus: a themed
  roster spawns seed-deterministically, the Act-1 entry is a depth-0 KillAll tutorial, a picked branch
  tears down the previous encounter (player persists), survive-on-timer, kill-map-boss ignores pack
  adds, the act boss advances the act, node depth deepens boss health + damage (the live scaling
  driver), and a ThroneRoom applies its curse (doubling enemy damage) + opens the Rare-floor reward.
  Plus unit tests: `build_act_graph` determinism/invariants, the room-layout blob-port regression pin,
  the depth formula, and `warlord`/theme RON parse. The golden master stays **byte-identical** (the
  campaign never starts a run — encounter systems gate on `CurrentEncounter`/`RunState`).
- Phase 7.5 (done): the UI is presentation-only (never headless), so these drive its **logic** — the
  state flows, the run-reset primitive, and the merchant ops — through the real input paths.
  `tests/game_flow.rs` — "player death enters GameOver" (the declared death behavior change),
  "restart after death boots a fresh run" (a clean entity census incl. the dead player's ability
  instances; deterministic under a fixed seed), "Esc toggles pause and preserves combat events" (the
  freeze contract, now for `Paused`), "pause does not tick the world", "character-select starts the
  chosen hero" (Menu → CharacterSelect → Mage → the entry encounter). `tests/merchant.rs` — "merchant
  remove uninstalls talent and hook", "merchant trade offers higher rarity". `tests/combat.rs::
  player_despawns_on_death` gained a `GameOver` assertion. The golden master stays **byte-identical**
  (the whole UI is in `PresentationPlugin`; every logic touchpoint is inert on the campaign path).
  Screens themselves are verified manually on Windows (WSL has no GPU).

Keep each scenario one mechanic; put cross-system drift detection in the campaign baseline.

## The compat agent

`/compat-check` (`.claude/skills/compat-check/SKILL.md`) runs the whole ladder — build,
warnings, unit tests, scenarios, golden master — then classifies failures as regression vs.
declared change by reading the diff and CHANGELOG, and reports with file/line references.
It is the on-demand backward-compatibility gate to run after each phase (or before a commit).

## Balance testing (later stages)

The same harness is the substrate for balance evaluation. Enemies are now data-driven with a
scaling model (Phase 5 — `EnemyDef` + `resolve_enemy_stats(def, depth)`; `Sim::spawn_enemy_at_depth`
drives the curve), so Stage 3's sweeps are unblocked; they become fully useful once encounters exist
(Phase 7):

- an `arena` binary running hero × build × encounter × seed sweeps at high speed,
- `BotPolicy` implementations as the stand-in player (deterministic, comparable),
- JSONL metrics (clear time, DPS in/out, TTK per enemy type, level curve) consumed by a
  balance-analyst agent that ranks builds, flags outliers, and proposes RON tuning diffs.
