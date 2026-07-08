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

**Phase 8 baseline move.** The baseline was regenerated once in Phase 8, for the `RunRng` algorithm
switch (`SmallRng` → `rand_chacha::ChaCha8Rng`, needed so a resumed run can restore its exact RNG
stream position — see CHANGELOG "Phase 8" and architecture-plan §8.11). This is also a **stronger**
portability guarantee than before: `SmallRng` is explicitly documented as *not* stable across `rand`
versions or platforms (a future `rand` bump could silently shift it), whereas `ChaCha8Rng`'s output is
a documented, versioned, portable stream. Every Phase-8 step after the switch re-verified
byte-identical against this baseline.

**Phase 9.2 baseline moves (×3).** Phase 9.1 stayed byte-identical throughout (its primitives were
inert). Phase 9.2 made three isolated, declared moves as its content went live: (1) base_stats
application (DK 100→200 hp — a clean hp-only diff), (2) Companion becoming a real active minion
(a wider diff — xp/enemies/statuses/position all shift once it's contributing real DPS), (3) the
combined rest-of-kit batch (Heart Strike, Abomination Limb, Purgatory, Bone Shield, and every new
talent tree — not separable from each other by the time they're all wired into the same default DK
loadout). See CHANGELOG "Phase 9.2" for the full attribution of each move.

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
- Phase 8 (done): `tests/persistence.rs` — "RunState syncs abilities/talents/timer into RunState at a
  node transition" (the `sync_run_state` prerequisite every other guarantee rests on), "save→resume
  reconstructs a live run" (health/level/abilities/talents/node/act all match), "resume continues the
  RunRng stream exactly" (the D1 headline: two independent resumes of byte-identical saved data roll
  an identical roster), "resume with no save falls back cleanly" (stays in the menu, no panic).
  `tests/meta.rs` — "a locked hero pick is refused" (against a deliberately-locked hero — none are
  locked by default, D3), "a defeat and an Act-3 victory each append a scored RunRecord" (the latter
  also caught the `enter_merchant` crash bug below), "the scoreboard's data source sorts run_history by
  score descending". `tests/game_flow.rs` gained "boot reaches Login then Menu" and "Resume Run from
  the main menu enters InRun with the saved run". Plus unit tests: `RunRng`'s serialize/restore
  mid-stream contract + ChaCha8 determinism, `RunState`/`MetaState` RON round-trips, the save-path
  resolver + corrupt/missing-file fallback, the score formula across act/node/level/victory/time, and
  `hero_is_unlocked`. **The golden master moved once** (8A's declared RNG-algorithm regeneration);
  every step after it verified byte-identical. Two pre-existing bugs were found and fixed by this
  phase's new coverage (not deferred): `enter_merchant`'s bare `Res<CurrentEncounter>` panicked on the
  Act-3 victory path (no prior test reached it), and a same-frame talent re-install onto a resumed
  player could race `attach_talent_components` (fixed with a synchronous attach +
  `Without<AcquiredTalents>`). The Login/Scoreboard screens are presentation-only, verified manually on
  Windows.

- Phase 9.1 (done): shared content-pass primitives, all inert until Phases 9.2+ wire real content
  into them (see architecture-plan §8.12). `tests/shields.rs` — a shielded actor takes no health
  damage until the pool is spent, then spills the remainder; grants stack additively. `tests/
  forced_movement.rs` — a grip impulse pulls an entity toward a point; the impulse expires and stops
  driving the entity; a knockback impulse stops at a wall (the per-axis `TileMap` slide still
  applies). `tests/charges.rs` — a `Charges` component syncs into the HUD's `ClassResource` bar with
  no other wiring. `tests/combat.rs` gained: a forced 100% crit multiplies damage by the default
  crit_mult; no crit talent means no crit; +100% attack speed halves Death Strike's observed
  cooldown. `tests/hero_stance.rs` gained: Shift/Space (`InputSlot::Movement`) triggers a bound
  dash end-to-end. Plus unit tests: `drain_absorb`'s spill-over math; `roll_crit`'s zero-chance
  short-circuit (the byte-identical guarantee) and always-succeeds-at-100% cases; the universal
  crit/attack-speed stat baseline's neutral defaults + a general talent reaching an undeclared stat;
  `Charges::gain`/`spend_all`; the `blink` behavior's pure targeting logic; `dash.ability.ron`'s
  parse. The golden master stayed **byte-identical** — no shipped ability/talent/enemy references
  any of the five new primitives.
- Phase 9.2 (done): the BDK kit closeout (architecture-plan §8.13) — 8 new scenario files, one per
  new ability/talent-tree area: `tests/heart_strike.rs`, `tests/abomination_limb.rs`,
  `tests/purgatory.rs`, `tests/bone_shield.rs`, `tests/amz_talents.rs`, `tests/blood_boil_talents.rs`,
  `tests/bdk_class_passives.rs`, `tests/companion.rs`. A recurring pattern across most of them: an
  auto-cast ability granted mid-test needs care around the exact frame its `AbilityInstance` becomes
  visible to `execute_ready_abilities` (`grant_level_1_abilities`/`spawn_unlocked_ability` run
  `.after(CombatSet::Death)`, so an instance granted this frame is first cast-eligible only the
  *next* frame) — spawn any target enemy **before** stepping past the grant, not after, or an
  early whiff can consume the cooldown before the scenario's own checks run. `Sim::disable_companion()`
  (new helper) isolates ~10 pre-existing tests from Companion's now-real incidental damage. The
  golden master moved **three times** this phase (base_stats; Companion; the combined rest-of-kit
  batch) — see the CHANGELOG "Phase 9.2" section for exactly what each regen covers.
  **Known open issue:** after regen #3, `campaign_is_reproducible_within_a_build` started failing
  intermittently (~1 run in 3) — several real scheduling races were found and fixed (see the
  CHANGELOG entry and architecture-plan §8.5's new row), but one more divergence source remains
  unidentified. The baseline is deliberately **not** regenerated a fourth time until this is fully
  resolved — `campaign_matches_golden_baseline` currently fails against the stale regen-#3 baseline,
  a known, expected state, not a new unexplained regression.
- Phase 9.3 (done): the Paladin, the arc's first brand-new hero (architecture-plan §8.14) — one new
  file, `tests/paladin.rs` (9 scenarios): Hammer of Justice's primary-full/cleave-half split + a
  clean whiff (no primary in arc); Flash of Light heals only once its `cast_time` channel completes,
  not the instant it's cast; the overheal→shield talent (computed from pre-heal `Health.current`,
  since `apply_heal` clamps to max and can't be un-clamped after the fact); Spinning Hammer's exact
  2:1 marked-vs-unmarked damage ratio — **the pattern worth reusing**: pin both a marked and an
  unmarked control entity to the IDENTICAL point on the orbiting hammer's path (a direct
  `WorldPosition` write via `sim.world_mut()`, not `spawn_grunt`'s tile placement) so both are swept
  the exact same number of times regardless of sweep-timing geometry, isolating the multiplier being
  tested from "how many times did it get hit" noise; Smite applies holy_mark (the grant path) and
  its `smite_spawns_consecrated_rare` talent drops the zone under the TARGET, not the caster;
  `consecrated_ground_slow_common` applies the new `consecrated_slow` status to zone occupants; and
  the headline `selecting_paladin_unlocks_its_own_band_kit_not_the_death_knights` — drives the REAL
  `Sim::request_start_run("paladin", seed)` path (not the lighter `Sim::set_hero` test shortcut,
  which deliberately skips re-applying `base_stats`/band pools) and levels to 4, asserting the
  Paladin's own three band abilities land and NONE of the BDK's five do — the regression test for
  the hero-aware `init_level_flow` fix (a real, previously-undiscovered bug — see the CHANGELOG
  entry). One timing gotcha specific to auto-cast + status-application ordering, beyond Phase 9.2's
  already-documented grant/whiff one: an `ApplyStatusEvent` sent via `sim.apply_status(...)` is only
  QUEUED that frame — the `StatusEffectInstance` doesn't exist until `StatusSet::Tick` runs LATER
  THAT SAME FRAME, which is AFTER `CombatSet::Damage` (where ability casts read marks/statuses). A
  scenario that grants an ability and applies a status in the same setup block, then steps once,
  can have that first cast see a NOT-YET-APPLIED status — apply the status and let it settle
  (`sim.step`) BEFORE granting the ability whose first auto-cast needs to observe it, mirroring how
  Phase 9.2's own grant-then-spawn-target ordering rule works for the opposite reason (a target that
  doesn't exist yet vs. a status that hasn't landed yet — same underlying "which frame is this
  actually visible on" trap). Plus unit tests (see architecture-plan §8.14) for every new ability/
  talent RON parse, the three new behaviors' pure targeting math, and the `DamageFraction`/
  `SecondaryHits` bake logic. Golden master **byte-identical** (the campaign is the BDK bot and
  never references Paladin content) — `campaign_matches_golden_baseline` is Phase 9.2's own tracked,
  unchanged divergence, not investigated further this sub-phase.

- Phase 9.4 (done): the Druid, the arc's second new hero (architecture-plan §8.15) — one new file,
  `tests/druid.rs` (10 scenarios). A cooldown-timing gotcha beyond the two above, specific to testing
  an ability twice in one scenario: `sim.trigger_ability(...)` still respects `AbilityCooldown` (a
  manually-sent trigger is NOT a cheat-cast) — a scenario that casts Scratch, then wants to cast it
  AGAIN to compare an Enhanced vs. non-Enhanced outcome must `sim.step_seconds(cooldown + margin)`
  between the two triggers, or the second one silently no-ops (cooldown not ready). A related trap
  the fixed version of that same scenario hit next: if a DoT (bleed) from the FIRST cast is still
  ticking, waiting out the cooldown lets it land an extra tick BEFORE the health reset meant to
  isolate the second cast's own damage — reset health AFTER the wait, immediately before the second
  trigger, not before. Also: `Sim::set_charges(entity, current, max)` inserts a `Charges` component
  directly regardless of the entity's actual hero identity, so most scenarios exercise the Enhanced
  state on the default DK-identified sim player (mirroring the Paladin file's own
  grant_ability/grant_talent-on-the-DK-player pattern) — only the stance-swap-casts-basic and
  hero-band-pool scenarios need a real Druid identity (`Sim::set_hero`/`Sim::request_start_run`).
  `campaign_matches_golden_baseline`'s divergence was independently reverified unchanged this
  sub-phase (reproduced byte-for-byte via `git stash` on a clean pre-9.4 checkout before any Druid
  code landed) rather than assumed unchanged — worth doing again for any sub-phase that touches
  several systems' schedules at once (new behaviors, a new AI steering branch, a rescheduled
  HUD-sync system all landed together here). Plus unit tests (see architecture-plan §8.15) for every
  new ability/talent RON parse, `LeapToTarget`'s two selection modes, `Bloom`'s pickup signal, and
  `Charges::spend_one`.
- Phase 9.5 (done): the Mage, the arc's fourth and final class kit (architecture-plan §8.16) — one
  new file, `tests/mage.rs` (10 scenarios): Frostbolt's frost-charge generation fires only against an
  ALREADY-frostbitten target (the first hit that applies frostbite grants nothing); Fireblast's
  explode-on-impact talent; Flamewrath's nearest-ablaze-target explosion + blaze consumption, and the
  no-consume trade-off talent; Flamestrike's per-blazed-enemy damage bonus applying to every hit;
  Frost Impale's channel firing no damage until completion, then scaling exactly by the number of
  frost charges consumed; both Frostbite-passive kill-reactive talents; the headline hero-band-pool
  regression test. **A real, previously-latent scheduling bug found by this sub-phase's own new
  content** (not a Mage-specific bug, but exposed by it) — see architecture-plan §8.16 and CHANGELOG
  "Phase 9.5" for the full account: `bone_shield_on_kill`/`overkill_leech_on_kill`'s own doc comments
  had wrongly assumed reading a dying `Enemy` was order-agnostic relative to `enemy_death`'s despawn
  (Bevy auto-inserts a sync point right after any Commands-issuing system, so an unordered same-set
  reader can lose the tie-break and see the entity already gone — there is no despawn-visibility
  grace period within a set at all). Caught by `tests/bdk_class_passives.rs` — a pre-existing,
  unrelated BDK test — the moment the Mage's two new Death-set readers shifted the tie-break order.
  Fixed at the root (`.before(enemy_death)` on every Death-set reader of a dying `Enemy`), not just
  patched for the two new systems. A second, smaller instance of the same class of bug required
  strengthening the Phase 9.4 `sync_charges_to_class_resource` pin from `.after(CombatSet::Damage)`
  to `.after(CombatSet::Death)` (the Mage's frost-charge-on-kill grant is the first `Charges` mutator
  living in `CombatSet::Death`). `campaign_is_reproducible_within_a_build`'s own already-documented
  intermittent flake (~1 run in 3, §8.5) was directly observed during this validation (fail once,
  pass twice across three consecutive runs) — consistent with the tracked rate. Plus unit tests (see
  architecture-plan §8.16) for every new/updated ability/talent RON parse, `TargetedBurst`'s pure
  targeting math, and the three new Pre hooks.

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
