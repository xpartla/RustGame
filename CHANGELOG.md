# Changelog

---

## [Unreleased] — Architecture, Scaffold, Phases 0–4 & Testing Infrastructure
_2026-07-04/05 — commits `5067dfb` (scaffold) → `2963e56` (phase 0) → `894452d` (phase 1) → `87b24ae` (docs) → `bc9d1d2` (phase 2) → testing infrastructure (stages 0–2) → `884a406` (test suite) → `7004259` (phase 3) → `be8e6ba` (phase 3.1) → phase 4 (hero/stance + Mage)_

### Architecture
- Wrote `docs/architecture-plan.md` — full foundational architecture covering all 9 subsystems:
  ability execution, talent/modifier stack, hero/stance system, status effects, persistent zones,
  leveling flow, act graph, enemy AI framework, and dual-scope persistence (run vs. meta).
- Resolved all five blocking design questions: ThroneRoom is the kiss/curse room (mandatory
  curse modifier + rare talent reward + distinct layout); no meta-progression between runs beyond
  hero unlocks and scoreboard; local-only persistence for now; Death Knight and Paladin have
  no stance (Q unbound); scaffold generation confirmed as next deliverable.
- Decided on data-driven content via RON assets for all ability, talent, hero, enemy, theme, and
  status effect definitions. New content = new file, not a code change.
- Decided on a hybrid talent system: numeric modifiers as pure data processed by a modifier
  stack; behavior-rewriting talents as registered code hooks referenced by ID from RON.

### Scaffold — New Modules (not yet wired into `main.rs`)
- `src/ability/` — `AbilityDef` RON asset schema, `BehaviorRegistry` + `AbilityHook` trait,
  `AbilityInstance` child-entity component, 8 built-in behavior stubs (melee cone, projectile,
  periodic zone, dropped zone, orbiting, leap to target, channel while moving, summon),
  `resolve_params` skeleton, execution system stubs.
- `src/talent/` — `TalentDef` RON asset schema, `AcquiredTalents` + `ActiveHooks` components,
  modifier stack (`resolve_params` pure function), offer generator with uniqueness constraint
  checks (`Stack(N)`, `Exclusive`, `MutuallyExcludes`), merchant operation stubs.
- `src/hero/` — `HeroDef` RON asset schema (stance slots, band pools, class passive pool),
  `ActiveStance` component, `InputSlot` enum, input-slot-to-ability resolution system stub,
  stance-swap system stub.
- `src/status/` — `StatusEffectDef` RON asset schema (stacking rules, element cancellation
  via `removed_by_tags`), `StatusEffectInstance` child-entity component, `ApplyStatusEvent` /
  `RemoveStatusEvent`, tick and cross-interaction system stubs.
- `src/zone/` — `PersistentZone` entity, `ZoneAnchor` (fixed or follow-entity), 
  `PlayerZonePresence` resource rebuilt each frame as the single spatial query cache for
  zone-presence checks across all systems.
- `src/run/` — `RunState` resource (full resumable run state), `RunRng` (seeded `SmallRng`
  — the only source of seed-deterministic randomness; non-deterministic systems use
  `thread_rng`), encounter-transition system stub.
- `src/progression/` — `LevelUpFlowState` (two-phase: `AbilityUnlock` → `TalentChoices`),
  level-up consumer system stub, talent offer / ThroneRoom reward system stubs.
- `src/meta/` — `MetaState` resource (hero unlocks, scoreboard, resumable run save slot,
  deliberately decoupled from `RunState`), local-file persistence stubs.
- `src/enemy/assets.rs` — `EnemyDef` + `ThemeDef` RON asset schemas.
- `src/enemy/behavior.rs` — `AiBehaviorRegistry` + `EnemyAiHook` trait, 3 AI stubs
  (melee chaser, ranged caster, stationary).
- `src/world/graph.rs` — `ActGraph`, `EncounterNode`, `EncounterType` (Map, BossRoom, ActBoss,
  ThroneRoom, Merchant), `ObjectiveType`, `RoomModifierDef`.
- `src/world/generator.rs` — per-encounter-type layout dispatch; throne room gets its own
  geometry generator distinct from the normal room pool.

### Scaffold — Example RON Assets
- `assets/heroes/blood_death_knight.ron` — full HeroDef for the BDK: stats, ability pools,
  class passive pool, default stance slot mapping.
- `assets/abilities/death_strike.ron`, `dnd.ron`, `companion.ron` — three BDK level-1
  abilities with base params and hook declarations.
- `assets/talents/death_strike_leech_common.ron` — numeric `MultiplyAdd` modifier, `Stack(3)`.
- `assets/talents/death_strike_range_common.ron` — numeric range modifier, `Stack(3)`.
- `assets/talents/death_strike_bone_shield_epic.ron` — behavior hook talent, `Exclusive`.
- `assets/talents/blood_boil_dnd_range_rare.ron` — zone-conditional behavior hook, `Exclusive`.
- `assets/enemies/grunt_placeholder.ron` — target schema for the existing Grunt archetype.
- `assets/themes/sand_dune.ron` — full enemy and boss pool for Sand Dune theme.
- `assets/status_effects/bleed.ron`, `blaze.ron`, `frostbite.ron`, `holy_mark.ron`,
  `root.ron`, `stun.ron` — all six status effects defined, with element cancellation
  cross-references encoded in the files.
- `assets/room_modifiers/enemies_deal_double_damage.ron`, `no_regen.ron`,
  `player_slowed.ron` — three example ThroneRoom curse modifiers.

### Phase 0 — Foundation (complete)
Backward-compatible groundwork from the migration plan (architecture-plan.md §7). No visible
gameplay change: the app still boots straight into gameplay and plays identically.
- `src/core/events.rs` — `DamageEvent` gains `tags: Vec<DamageTag>` field. All existing
  callers updated to pass `vec![]`; field is currently unused (consumed in Phase 3).
  `DamageTag` enum added: `Physical`, `Fire`, `Frost`, `Holy`, `Shadow`, `Arcane`.
- `src/core/sets.rs` — `StatusSet { Tick, CrossInteract }` declared for the eventual combat
  chain (`Damage → Apply → Tick → CrossInteract → Death`). Wired into `CorePlugin` in Phase 3.
- `src/game/state.rs` (new) — `GameState` enum: `Menu`, `CharacterSelect`, `InRun`, `Paused`,
  `GameOver`, `TalentPicker`, `Merchant`. Registered with `init_state`; defaults to `InRun` so
  the app boots straight into gameplay (no menu exists yet — Phase 8 flips the default).
- System gating — every gameplay-simulation system (input, movement, combat, enemy AI,
  spawning, XP, pickups) now runs under `run_if(in_state(GameState::InRun))`. A no-op today
  (default state is `InRun`), but future `Paused`/menu/overlay states will freeze the world
  without editing any individual system. Render-sync, camera, and debug gizmos stay ungated.
- `src/run/rng.rs` wired in — `RunRng(SmallRng)` inserted at startup and consumed by
  `generate_map`, replacing `rand::thread_rng()`. Seeded from OS entropy per launch for now
  (preserves the prototype's per-launch map variation); Phase 7's run-start flow supplies the
  real reproducible/​resumable seed. Only `run::rng` is compiled — the rest of `src/run/`
  (state, plugin, transitions) stays scaffold-only until Phase 7.
- `Cargo.toml` — enabled `rand`'s `small_rng` feature (required by `SmallRng`).
- Removed stray `CORE_DRAFT.md:Zone.Identifier` (a Windows download artifact, not a real file).

### Phase 1 — Ability System (complete)
Replaces the two hardcoded prototype attacks with a data-driven ability pipeline. Left-click now
casts the Blood DK's Death Strike, loaded from a RON file.
- Content pipeline: added `serde` + `ron`. `AbilityDef` is now a Bevy asset with a custom
  `AbilityDefLoader` for `*.ability.ron` files — a distinct extension so the talent/hero/enemy
  loaders added in later phases never collide on plain `.ron`. Ability RON files renamed to
  `*.ability.ron` accordingly.
- `ability/behavior.rs` — `BehaviorRegistry` + `AbilityBehavior` trait. A behavior receives a
  read-only `AbilityContext` (owner, origin, aim, candidate targets) and pushes `AbilityEffect`s
  (Damage / Heal / VFX); the execute system is the only code that touches `Commands`/`EventWriter`,
  keeping behaviors pure and unit-testable. Implemented `MeleeCone` (Death Strike): cone hit-test
  + leech + hitbox VFX flash, reproducing the old `player_arc_attack` math from RON params.
- `ability/systems/execute.rs` — `tick_ability_cooldowns` + `execute_ready_abilities`, chained in
  `CombatSet::Damage`. Trigger-driven; resolves the `AbilityDef` via an `AbilityLibrary` (id→handle),
  and skips gracefully while an asset is still loading or if a behavior id isn't registered. Its
  private `apply_effects` helper is the sole write point that drains the effect buffer: `Damage` →
  `DamageEvent` (tagged `Physical`), `Heal` → the prototype's existing `HealEvent`/`apply_heal` chain
  (this is how Death Strike's leech heals the caster — no new heal path), and `ConeVfx` → a transient
  `Projectile` + `ArcHitbox` + `Lifetime` entity, reusing the prototype's hitbox-gizmo path so the
  existing debug renderer draws the cone. A cast is suppressed until `Facing` is non-zero (no attack
  before the first mouse-aim), and candidate `EnemyTarget`s are gathered once per frame for all casts.
- `ability/systems/resolve_params.rs` — Phase-1 identity resolution (base params verbatim). The
  talent modifier stack layers on top in Phase 2.
- Ability instances: `AbilityInstance { def_id, owner }` + `AbilityCooldown` per unlocked ability
  (each a separate entity; `AbilityCooldown::new` starts ready so an ability can fire immediately).
  Death Strike is granted at spawn by a Phase-1 stub (`grant_starting_abilities`); Phase 2 moves
  this to progression-driven `UnlockAbilityEvent`. Cooldown duration is re-read from the resolved
  `"cooldown"` param on every cast, so future cooldown talents take effect on the next fire.
- Reserved scaffolding left in place (not deleted; `#[allow(dead_code)]` until its phase):
  `StanceGate` (Phase 4 stance filter), `AbilityHookState` (per-ability hook counters), and
  `UnlockAbilityEvent` (Phase 2 progression) in `ability/components.rs`, plus deserialized-but-unread
  `AbilityDef` fields (`unlock_schedule`, `hooks`, `talent_pool`, `display_name`). The scaffold's
  separate `AbilityHook` trait and `HookRegistry` were dropped from `behavior.rs` — the read-only
  context + effect-buffer model above replaces the old `&mut AbilityContext` placeholder, and talent
  hooks return with Phase 2. (A few module comments still name `HookRegistry`; cleaned up when Phase 2
  reintroduces it.)
- Input: LMB → `TriggerAbilityEvent("death_strike")` via a Phase-1 stub in `player`
  (`player_ability_input`). The stance-aware hero indirection (hero module) stays a Phase-4 concern
  — deliberately not wired yet, to avoid pulling the talent module into the build early.
- Activation: `AbilityPlugin` is now added to `GamePlugin`'s plugin tuple and `mod ability;` to
  `main.rs`, so the scaffold module (flagged "not yet wired into `main.rs`" above) is live. A Startup
  `load_ability_defs` loads a fixed id→path list (`death_strike`, `dnd`) into `AbilityLibrary`; `dnd`
  loads but its `dropped_zone` behavior isn't registered yet, so triggering it just warns and skips —
  exercising the same graceful-degradation path as an asset that is still mid-load (Phase 6 registers
  the behavior). Only `melee_cone` is registered in `BehaviorRegistry` this phase.
- Removed the prototype attacks: deleted `player/systems/attack.rs` (`player_circle_attack` /
  `player_arc_attack`), their Space/V bindings, and the now-dead attack constants — `ARC_BASE_DMG`,
  `CIRCLE_BASE_DMG`, `ATTACK_SPAWN_DISTANCE`, `ATTACK_HITBOX_RADIUS`. Only `ATTACK_LIFETIME` survives
  in `constants.rs`, repurposed to time the transient VFX flash alone (damage/range/cooldown now live
  in the ability RON). The radial-burst shape was a prototype placeholder (not part of the BDK kit)
  and was dropped; the cone attack lives on as Death Strike.
- Tests (headless, `cargo test`): RON round-trip of `death_strike`/`dnd`, and `MeleeCone`
  range/arc/leech logic — 4 passing. The full in-game loop is still unverified in WSL (GPU backlog).

### Phase 2 — Talent System (complete)
Brings the `talent`, `progression`, and a new `ui` module online (added to `main.rs` and
`GamePlugin`). Numeric talents now flow through a real modifier stack, level-ups drive a
two-phase ability-unlock → talent-choice progression, and a minimal on-screen picker lets the
player choose 1 of 3. Validated on the Blood DK's Death Strike.
- Content pipeline: `TalentDef` is now a Bevy asset with a `TalentDefLoader` for `*.talent.ron`
  (a distinct extension mirroring `*.ability.ron`, so the ability/talent loaders never collide on
  plain `.ron`). Added `serde::Deserialize` to `TalentDef` and its sub-types (`TalentRarity`,
  `UniquenessConstraint`, `TalentEffect`, `StatModifier`, `ModOp`). The four talent RON files were
  renamed to `.talent.ron`; a third numeric Death Strike talent (`death_strike_damage_common`,
  +20% damage, `Stack(3)`) was added and wired into `death_strike.ability.ron`'s `talent_pool` so
  offers present three working numeric options.
- `TalentLibrary` (id → `Handle<TalentDef>`) mirrors `AbilityLibrary`; a Startup `load_talent_defs`
  loads the five talent files. Ids with no loaded `TalentDef` self-filter everywhere, so
  class-passive / band references without RON files contribute nothing until their content pass.
- Modifier stack — `talent/modifier.rs::resolve_params`: gathers each acquired `Modifier`-effect
  talent scoped to the fired ability (or global `None`), stacking `(base + Σadd) * (1 + ΣmultAdd)`
  with `Override` applied last. A `Stack(N)` talent taken `count` times contributes its modifier
  per copy. Split into a pure `apply_modifiers` core (no ECS/assets) for direct unit testing.
  Replaced the Phase-1 identity resolver: `ability/systems/resolve_params.rs` was **deleted** and
  `execute_ready_abilities` now calls the talent stack with the caster's `AcquiredTalents`
  (`Option`, empty fallback for non-player casters) plus `Assets<TalentDef>` + `TalentLibrary`.
  Cooldown is still re-read from resolved params per cast, so cooldown talents would apply live.
- Talent state on the player: `AcquiredTalents` + `ActiveHooks` are attached on `Added<Player>` by
  the talent plugin (keeps the `player` module decoupled). `install_acquired_talent` applies a
  `TalentAcquiredEvent` (adds to `AcquiredTalents`; a `Behavior` effect also pushes its `HookId`
  into `ActiveHooks`); `uninstall_removed_talent` mirrors it for the merchant path (Phase 8). Both
  run ungated by state so an event emitted from the `TalentPicker` overlay is not frozen with the
  `InRun` world.
- Offer generation — `generate_offer`: samples up to 3 distinct eligible talents (`choose_multiple`)
  from the caller-built pool using `RunRng` (seed-deterministic). `is_eligible` enforces
  `Stack(N)` / `Exclusive` / `MutuallyExcludes` and an optional rarity floor (per `OfferContext`).
- Progression flow — `LevelUpFlowState` is now a `Resource`, inserted at startup with the BDK band
  pools shuffled by `RunRng` (a Phase-2 stub; Phase 4 sources them from `HeroDef`). Two phases:
  `AbilityUnlock` (L2–L6 draw one band ability per level, 2/3 pool then 4/6, emitting
  `UnlockAbilityEvent`) → `TalentChoices` (L7+ owe a talent choice). `handle_level_up` consumes
  `LevelUpEvent` after `gain_experience`. An `owed_choices` backlog + **lazy** offer generation
  (`refill_offer`) keep multi-level-in-one-frame and uniqueness correct (each offer reflects the
  prior acquisition). `handle_talent_choice` reads `1/2/3` (emit `TalentAcquiredEvent`) or `Esc`
  (decline); `refill_offer` closes the overlay once the backlog drains.
- Unified ability-grant path: registered `UnlockAbilityEvent`; `spawn_unlocked_ability` spawns one
  `AbilityInstance` per unlock (idempotent). The Phase-1 `grant_starting_abilities` stub became
  `grant_level_1_abilities`, emitting `UnlockAbilityEvent` for the hardcoded BDK L1 list. Band
  abilities (`blood_boil`, `heart_strike`, …) unlock as **inert** instances — no registered
  behavior, no input binding, no auto-cast yet — until their own phases.
- `GameState::TalentPicker` is now live (entered from the level-up flow). Because the whole
  gameplay simulation is already gated on `InRun`, the world freezes behind the overlay for free.
- New `ui` module (owns no data). `UiPlugin` + `ui/screens/talent_picker.rs`: a `bevy_ui` overlay
  spawned `OnEnter(TalentPicker)`, its option rows re-rendered on offer change (showing
  `display_name` + rarity resolved via `TalentLibrary`), torn down `OnExit`. Uses Bevy's built-in
  default font (no font asset). Input stays in `progression`, per the plan's "ui reads, owns nothing".
- Debug affordance (dev builds only): `debug_force_level_up` — pressing `L` grants exactly enough
  XP to reach the next level, so the talent flow is reachable without grinding kills.
- Scope boundaries held: behavior-hook **execution** during a cast (`AbilityDef.hooks`) is still
  deferred until the first real hook lands (bone shield); `Behavior` talents install an inert
  `ActiveHook` for now. Merchant ops (Phase 8) and ThroneRoom rewards (Phase 7) remain unscheduled
  `todo!()`. Passive ability auto-cast-on-cooldown is out of scope.
- Tests (headless, `cargo test`): modifier-stack math (additive/multiplicative/override/scope),
  talent RON round-trips, `is_eligible` per uniqueness constraint + rarity floor, level-flow
  band→talent transitions, and a `resolve_params` integration test driving the full
  id → `TalentLibrary` → in-memory `Assets<TalentDef>` → modifier path — **20 passing** total.
  Remaining 3 build warnings are the pre-existing Phase 3 scaffolding (`StatusSet`, `DamageTag`).
  The on-screen picker itself is unverified in WSL (GPU backlog) — to be checked on the Windows build.

### Testing Infrastructure — Stages 0–2 (complete)
The agentic backward-compatibility setup from the testing plan (see docs/testing.md). The
gameplay simulation now runs headlessly (no window/GPU — works in WSL), a golden scenario
suite plus a golden-master campaign lock in current behavior, and a `/compat-check` skill +
`compat-tester` agent run the gate on demand.

**Stage 0 — headless foundation**
- Crate split into a library (`rust_game`, all game code) + thin windowed binary. Integration
  tests and the sim harness link against the lib; `cargo run` is unchanged. Domain module
  visibility flipped `pub(crate)` → `pub` so tests can reach components/events. Removed the
  empty vestigial `player/systems/combat.rs`. Side effect: the 3 dead-code warnings for
  Phase 3+ scaffolding (`DamageTag` variants, `tags`, `StatusSet`) are gone — `pub` items in
  a library count as public API. The build is now warning-free; treat any new warning as a
  finding.
- **Logic/presentation split.** `GameLogicPlugin` (the full simulation, no render deps) +
  `PresentationPlugin` (camera, UI, map rendering, debug gizmos, and new `attach_*_visuals`
  systems that dress logic entities with Transform/Mesh2d/material on `Added<T>`). Logic
  spawns carry data instead of meshes — new `EnemyAppearance` component holds the archetype's
  shape/radius/color. `GamePlugin` = logic + presentation; windowed behavior is unchanged
  (same z-layers, same schedules/gating for every moved system).
- `src/sim/` — the headless harness: `SimPlugins` (MinimalPlugins + States + Assets +
  manually-controlled `ButtonInput`), fixed-timestep clock (`TimeUpdateStrategy::
  ManualDuration`, 60 fps), caller-provided `RunRng` seed, single-threaded schedules for
  stable ordering. `Sim` wraps it with scenario helpers: step/press/tap, spawn enemies from
  archetypes (shared `enemy_bundle` with the timed spawner), trigger abilities, teleport/heal
  /damage, pause ambient spawners, swap in an empty bordered arena, map signature hashing.
- **Determinism fixes (behavior-affecting, found during harness work):**
  - `init_level_flow` now ordered after `generate_map` — both draw from RunRng in Startup and
    were previously unordered, so the same seed could produce different maps and band-shuffle
    orders between launches.
  - Enemy death drops roll on `RunRng` instead of `thread_rng` (drops are gameplay, per the
    RunRng rule; also makes kill scenarios reproducible). Ambient enemy/pickup spawners stay
    on `thread_rng` for now — documented in docs/testing.md; scenarios pause them.

**Stage 1 — golden scenario suite + golden master**
- `tests/` — 20 scenario/integration tests over the sim harness: harness boot smoke tests;
  movement (WASD speed, wall slide, border blocking); combat (Death Strike cone membership,
  damage, leech, cooldown gating, inert-behavior skip; grunt contact cadence; player death;
  kill credit → XP); Phase 2 progression (six-levels-in-one-frame band unlock burst, picker
  round-trip, decline, uniqueness filtering against the current backlog, damage/leech talent
  modifiers on real casts, seed-deterministic band draws); world/pickups (map determinism per
  seed, spawn-clear box, heal clamp, pickup radius).
- `tests/golden_campaign.rs` — golden master: a deterministic scripted bot plays 30 simulated
  seconds against scripted waves (chase nearest, cast on cooldown, kite when hurt, auto-pick
  talents); a per-second trace (hp/level/xp/enemies/abilities/talents/position) must match
  the committed `tests/golden/campaign_baseline.ron` exactly. `UPDATE_GOLDEN=1` regenerates —
  only for CHANGELOG-declared changes, committed together with the change (the baseline's git
  history is the behavior-change audit trail). A second test replays the campaign twice and
  asserts identical traces — the nondeterminism tripwire.
- Cargo: `[profile.dev.package."*"] opt-level = 2` (standard Bevy setup) so dependency code is
  optimized in dev/test — the campaign runs at usable speed; our code stays fast-compiling.

**Stage 2 — the compat agent**
- `.claude/skills/compat-check/SKILL.md` — `/compat-check` runs the ladder (build + warnings →
  full test suite → golden master), then classifies each failure as REGRESSION (undeclared),
  DECLARED CHANGE (explained by this changelog; baselines may be updated, with justification),
  or NONDETERMINISM (never papered over by regeneration).
- `.claude/agents/compat-tester.md` — subagent definition so the check can run delegated.
- `docs/testing.md` — harness design, layer map, baseline/back-tracing procedure, known
  nondeterminism, and the per-phase definition-of-done (each phase lands with its own golden
  scenarios).

**Phase 2 fixes (found by verification + the new suite)**
- **Band abilities could be silently lost.** When several level-ups land in one frame and
  cross from the AbilityUnlock band into TalentChoices (e.g. a big XP grant), the
  `UnlockAbilityEvent`s were written the same frame the state flipped to `TalentPicker` — and
  `spawn_unlocked_ability` was gated on `InRun`, so the events expired unread. The
  grant/spawn pair now runs ungated (same reasoning as the talent install systems). Caught by
  the `six_levels_in_one_frame` golden scenario.
- **Backlog offers could sample stale talent state.** `refill_offer` had no ordering against
  `install_acquired_talent`, so with several owed choices the next offer could be generated
  before the previous pick landed in `AcquiredTalents` — letting an Exclusive/capped talent
  be offered (and taken) twice. Now explicitly ordered after the install system; locked in by
  the `offers_respect_uniqueness` scenario.

### Phase 3 — Status Effects (+ generic effect model, projectiles, auto-cast) (complete)
Brings the `status` module online (added to `main.rs`/`GameLogicPlugin`) and delivers the status
effect system end-to-end: DoTs, cross-element cancellation, crowd control, and the stat debuffs
that make them matter. Three planning decisions widened the phase (see `docs/phase3-plan.md`):
auto-cast was folded in, ability→effect became a fully declarative effect list, and the Mage
projectile abilities were pulled forward as faithful test vehicles. Implemented in six
compat-gated sub-steps (3A–3F); the golden baseline moved only on declared changes.

- **Generic effect model (3A).** Behaviors now resolve *targeting* (a `CastOutcome`: hits +
  primary + optional VFX + optional projectile) and the ability's declarative
  `effects: Vec<EffectSpec>` decides the *outcome*. `EffectSpec` = `Damage{amount,tags,target}` /
  `Heal` / `Leech{percent}` / `ApplyStatus{status,stacks,target}`, with `EffectTarget` ∈
  `AllHits | PrimaryHit | Caster`; numeric fields reference param keys so the talent modifier
  stack still reaches every number. One shared applier (`ability/effects.rs`:
  `resolve_effects` + `apply_resolved_effects`) drives both instant casts and projectile impacts.
  `MeleeCone` was rewritten to return hits (damage/leech moved to `death_strike.ability.ron`'s
  effect list); the prototype's `AbilityEffect` enum was removed. `AbilityDef.effects` is
  `#[serde(default)]` so un-migrated abilities parse inert. **This step was a pure refactor —
  Death Strike stayed numerically identical and the golden baseline was byte-for-byte unchanged.**
  `DamageTag` gained `serde::Deserialize` (first RON consumer).
- **Status core (3B).** `StatusEffectDef` is a Bevy asset with a `*.status.ron` loader
  (extension mirrors `*.ability.ron`/`*.talent.ron`) + `StatusLibrary`. The scaffold's opaque
  `on_*_hooks` sketch was replaced with a **declarative** schema — `tick: Option<TickSpec>`
  (interval/damage/tags), `move_speed_mult`, `damage_taken_mult`, `immobilize`,
  `suppress_abilities`, `removed_by_tags`, `removes_on_apply` — so the six built-ins need **zero
  Rust** per effect; a `hooks` escape hatch remains (empty) for a future code-driven effect, and
  the `StatusHookRegistry` is deferred until one lands. The six RON files were rewritten to the
  new schema (`bleed`/`blaze`/`frostbite`/`holy_mark`/`root`/`stun`, renamed `*.status.ron`).
  Each active effect is a top-level `StatusEffectInstance{def_id,target,source,timer,tick_timer}`
  entity (the `target` field is the query key — no hierarchy, mirroring `AbilityInstance.owner`);
  `despawn_orphaned_status` reaps instances whose target died (Bevy 0.16 `despawn()` is
  non-recursive). Lifecycle systems: `apply_status_effects` (honors `StackingRule`
  RefreshOnReapply / StackCapped(n) / StackUnlimited + `removes_on_apply`), `tick_status_effects`
  (DoT DamageEvents carrying the effect's `source` and element tags; despawns on expiry),
  `apply_cross_interactions` (DamageEvent tags → RemoveStatusEvent, deduped), `remove_status_effects`.
  `EffectSpec::ApplyStatus` emits `ApplyStatusEvent`. `StatusSet::{Tick,CrossInteract}` was wired
  into the combat chain (`Damage → Apply → Tick → CrossInteract → Death`).
- **CC & stat integration (3C).** New generic core components `MoveSpeedModifier(f32)`,
  `DamageTakenModifier(f32)`, `Immobilized`. `resolve_actor_status` folds each actor's active
  statuses into them (product of move/damage mults, any immobilize), inserting a component only
  when it deviates from neutral and removing it when it returns — so status-free actors never
  carry them. `apply_velocity` scales its integration *step* by `MoveSpeedModifier` and skips when
  `Immobilized` (scaling the step, not the stored velocity, keeps the enemy-AI lerp clean — this
  is a deliberate simplification of the plan's separate `apply_movement_status` system, avoiding a
  feedback bug). `apply_damage` multiplies incoming damage by `DamageTakenModifier`. Net effects:
  frostbite slows 0.8× and amplifies 1.1×; root/stun freeze movement (stun also flags
  `suppress_abilities`, whose consumer arrives with enemy AI in Phase 5). Status stat effects lag
  application/removal by one frame by design (documented in `docs/phase3-plan.md` §2.6).
- **Projectiles (3D).** The `projectile` module grew real motion + collision: `ProjectileMotion`
  (velocity/radius/pierce) + `ProjectilePayload` (source + baked effects + already-hit set).
  The `projectile` behavior returns a `ProjectileSpawn`; `execute` spawns the entity carrying the
  ability's baked effects; `move_projectiles` + `projectile_collision` (in `CombatSet::Damage`)
  integrate it and, on first contact (distance ≤ projectile radius + enemy radius), apply the
  payload via the shared applier — so a shot's damage/status land on *impact*, not at cast, and a
  pierce count is respected. Travelling projectiles reuse the `Projectile` marker + `Lifetime` so
  `projectile_lifetime` despawns them; only entities with `ProjectileMotion` are moved.
- **Demonstrator abilities (3D).** Added `fireblast` (Fire projectile → blaze), `frostbolt`
  (Frost projectile → frostbite), `scratch` (Physical cone → bleed) as standalone
  `*.ability.ron` files — **not yet class/stance-bound** (Phase 4 wires them to the Mage/Druid).
  They give the status system faithful test vehicles and make cross-element cancellation testable
  end-to-end (Fireblast clears frostbite; Frostbolt clears blaze; a blaze Fire tick clears
  frostbite emergently).
- **Auto-cast (3E).** `AbilityDef.activation` ∈ `Input` (default) | `AutoCast`; `auto_cast_abilities`
  emits a TriggerAbilityEvent for every ready AutoCast instance (before `execute_ready_abilities`
  in `CombatSet::Damage`). The blanket "no cast before aim" gate moved from owner-level to
  per-behavior via `AbilityBehavior::needs_aim()` — cones/projectiles still require a facing (and
  don't burn cooldown when aimless), self-centred novas don't. New `self_nova` behavior (hits all
  enemies within radius, no aim). **Blood Boil** (BDK L2/3 band ability, inert since Phase 2) went
  live as an auto-cast self-nova (6 dmg / radius 90 / 5% leech / 4s cd).
- **Declared golden-baseline changes.** 3B and 3D each shifted the golden campaign by a **sub-unit
  player-position drift** (verified across all 30 snapshots to be `px`/`py`-only — every gameplay
  field identical): wiring `StatusSet` into the combat chain (3B) and adding the projectile
  systems to `CombatSet::Damage` (3D) reordered the single-threaded tie-break for the loose
  movement systems. Per `docs/testing.md`, a reorder that shifts float behavior is a declared
  regeneration; both were regenerated after confirming no mechanical divergence. 3E regenerated
  the baseline for a **real** behavior change (Blood Boil now auto-casts) and enriched the master:
  the bot also throws Frostbolt (covering projectiles + frostbite) and the `Snapshot` gained a
  `statuses` column. The reproducibility tripwire still passes (no new nondeterminism — DoTs,
  projectiles, and status resolution carry no RNG). Post-Blood-Boil the bot reaches level 8 (was
  7) by 30s.
- **Tests (headless, `cargo test`): 66 passing** (was 43). Unit: 5 status RON round-trips,
  2 new `MeleeCone` targeting tests. Golden scenarios: `tests/status.rs` (bleed tick cadence +
  refresh + expiry, frostbite slow ≈0.8× + damage amp ×1.1, root/stun freeze-then-release,
  fire↔frost cancellation, emergent blaze-tick clear, DoT kill credit, unknown-id no-op),
  `tests/projectile.rs` (travel-then-hit, blaze/frostbite on impact, cross-element via real Mage
  abilities, bleed cone), `tests/autocast.rs` (Blood Boil auto-cast + cooldown gate, per-behavior
  aim gate). Build is warning-free.

### Phase 3.1 — Hardening (post-Phase-3 review) (complete)
A review pass over the Phase 3 implementation (2026-07-05) fixed one latent bug, closed the
review's structural findings, and filled the test-coverage gaps promised in
`docs/phase3-plan.md` §6 but not delivered with 3E. **The golden baseline did NOT move** — the
scheduling pin below matched the order the single-threaded tie-break already produced, and every
other change is behavior-neutral by construction (verified: `campaign_matches_golden_baseline`
passes against the unchanged committed baseline).

- **MovementSet pin (the Phase-3 "known follow-up").** New `MovementSet::{Intent, Integrate}`
  (core/sets.rs) chained ahead of the combat sets:
  `Intent (player_input, flow-field rebuild → enemy_follow → enemy_facing) → Integrate
  (apply_velocity → world_to_grid) → CombatSet::Damage → …`. Positions are no longer hostage to
  the scheduler's tie-break, so future phases can add Update systems without nudging the golden
  master's px/py (the cause of two benign regens within Phase 3).
- **Combat events survive overlay states (freeze-semantics fix).** Previously, an event written
  the frame an overlay opened could silently expire: every combat reader is InRun-gated and Bevy
  expires unread events after two frames — concretely, a DoT tick's `DamageEvent` (written in
  `StatusSet::Tick`, consumed by `apply_damage` the *next* frame) vanished whenever a level-up
  opened the TalentPicker in between. New `AddGameplayEventExt::add_gameplay_event`
  (core/events.rs): `DamageEvent`, `HealEvent`, `ApplyStatusEvent`, `RemoveStatusEvent` now
  advance their buffers only during InRun frames — the world freezes with pending events intact
  and they resolve on the first frame after resume; entering GameOver/Menu clears them so a dead
  run never leaks into the next. Input-intent events (`TriggerAbilityEvent`) still expire, and
  same-frame-consumed events (`GainXpEvent`, `LevelUpEvent`, `UnlockAbilityEvent`) stay standard.
  Locked by `tests/freeze.rs::dot_tick_pending_when_picker_opens_lands_after_resume`.
- **BUG FIX: same-frame double application of a status.** `apply_status_effects` spawns through
  `Commands`, so a second `ApplyStatusEvent` for the same (target, effect) in the same frame saw
  "no existing instance" and spawned a duplicate — two live instances of a `RefreshOnReapply`
  effect (double DoT), and `StackCapped` could overshoot its cap. Latent (no shipped content
  emits two same-status events in one frame yet; Phase 4 multi-appliers would have). Fixed with
  same-frame pending bookkeeping; locked by `same_frame_double_apply_keeps_a_single_refresh_instance`
  and the StackCapped scenario.
- **Hurtbox split (logic/presentation).** `projectile_collision` read `EnemyAppearance.radius` —
  a presentation-data component — as the gameplay collision radius. New generic
  `core::components::Hurtbox { radius }`: enemies get it in `enemy_bundle` (same archetype value,
  so behavior is identical), the player gets one (`PLAYER_RADIUS = 25`, extracted to constants.rs
  and shared with the visual circle) ready for enemy shots in Phase 5. Gameplay no longer reads
  any presentation component.
- **Coverage gaps closed** (promised in phase3-plan §6, missing from 3E): `StackCapped` /
  `StackUnlimited` scenarios (via a new `Sim::insert_status_def` synthetic-def helper — no
  shipped effect uses these rules yet), projectile pierce ×2 (pierce 0 stops at the first enemy;
  pierce 1 passes through — via new `Sim::set_ability_param` test knob), orphaned-status reaping
  on target death. New sim helpers: `insert_status_def`, `set_ability_param`,
  `hasten_status_tick` (aligns a DoT tick with another event without fragile frame counting),
  `grant_xp`.
- **Cleanups.** Stale Phase-1-era header comment in `ability/systems/execute.rs` rewritten for
  the Phase-3 model; stale `TODO(Phase 3)` in core/sets.rs removed; vestigial no-op assertion in
  tests/status.rs and dead `despawned` variable in projectile collision removed.
- **Debt made agent-visible.** Tech-debt register at `docs/architecture-plan.md` §8.5 (each item
  with its owning phase); new repo `CLAUDE.md` mapping the contract documents and register for
  future sessions; `/compat-check` now cross-checks findings against the register. Design
  decision recorded there: **projectiles passing through walls is accepted for now** (project
  owner, 2026-07-05); revisit during Phase 4 playtesting.
- **Tests: 73 passing** (was 66). Build remains warning-free.

### Phase 4 — Hero / stance system + Mage (focused vertical slice) (complete)
Brought the hero/class-identity + stance system online end-to-end and added the **Mage** as a
second playable class (Fire/Ice stances, Q swap). `HeroPlugin` joins `GameLogicPlugin`; the player
now carries `HeroIdentity` + `ActiveStance`, and input is resolved through the hero indirection
instead of the Phase-1 hardcoded LMB→Death-Strike stub. Scope was a deliberate **focused vertical
slice** (owner decision): the heavier Mage subsystems (frost-charge resource, Frost Impale, dash,
real absorb shields, code-driven hooks) are deferred with explicit revival triggers
(architecture-plan §8.6 / phase4-plan §7). See `docs/phase4-plan.md` for the full plan + as-built
notes. **The golden baseline did NOT move** — the Death Knight stays the default hero and the
campaign bot bypasses the input layer (verified: `campaign_matches_golden_baseline` passes against
the unchanged committed baseline; no regeneration).

- **Architecture: generic `DefLibrary<T>` (pays the §8.5 "Def-library triplication" debt, done at
  Phase 4 start).** `AbilityLibrary`/`TalentLibrary`/`StatusLibrary` were three near-identical
  copies (resource + `AssetLoader` + hardcoded Startup path list). New `core/def_library.rs`: a
  generic `DefLibrary<T>` resource, a `DefAsset` trait carrying each type's RON extension + load
  manifest, one `RonDefLoader<T>`, and an `App::register_def_library::<T>()` that wires
  asset+loader+resource+Startup-populate in one call. The three concrete libraries became type
  aliases (`pub type AbilityLibrary = DefLibrary<AbilityDef>;` …) so every call site and sim
  accessor compiled unchanged; `HeroDef` is registered the same one-line way instead of becoming a
  fourth copy. Pure refactor — baseline byte-identical.
- **Hero module live.** `HeroDef` gained `serde::Deserialize` (the scaffold omitted it) + `impl
  DefAsset` with the `hero.ron` compound extension; `assets/heroes/blood_death_knight.ron` renamed
  to `.hero.ron`. `HeroPlugin` (`hero/plugin.rs`) registers `HeroDef` and runs
  `resolve_input_to_ability` + `handle_stance_swap` before `CombatSet::Damage`, InRun-gated.
- **Input indirection replaces the Phase-1 stub.** `hero/systems/input_slot.rs::
  resolve_input_to_ability` reads `(HeroIdentity, ActiveStance)`, resolves the pressed slot via
  `HeroDef.stance_slots` (pure, unit-tested `resolve_slot`) — **LMB→Basic, RMB→Special** — and
  emits `TriggerAbilityEvent`. `player/systems/ability_input.rs` (the hardcoded LMB→death_strike
  stub) is deleted; `spawn_player` now inserts `HeroIdentity("blood_death_knight")` +
  `ActiveStance("default")`.
- **Stance swap (Q).** `hero/systems/stance.rs::handle_stance_swap` toggles `ActiveStance` between
  the hero's `stance_a`/`stance_b` and applies the *entered* stance's `swap_effect` status to the
  caster (new `#[serde(default)] swap_effect: Option<StatusEffectId>` field on `StanceSlotMapping`).
  No-op for non-stance heroes (`has_stance == false` — Death Knight).
- **HeroDef-sourced, deferred level-1 grant.** `grant_level_1_abilities` now reads
  `HeroDef.level_1_abilities` instead of a hardcoded array. Because the asset loads asynchronously,
  it is deferred: it fires once the player's `HeroDef` resolves and marks the player
  `Level1Granted` (new component). `sim::settle_assets`/`assets_loaded` wait for both the
  `HeroLibrary` handles and the grant, so `new_arena` returns with abilities in place. For the
  Death Knight this grants the identical `death_strike, dnd, companion` → baseline-neutral.
- **Mage content.** `assets/heroes/mage.hero.ron` (Fire/Ice, `resource_model: None`,
  `level_1_abilities: [fireblast, frostbolt]`); the existing Phase-3 demonstrators Fireblast/
  Frostbolt are now bound as the Fire/Ice **Basic** abilities. Stance-swap effects reuse the status
  system (no new machinery): `boots_of_fire.status.ron` (Ice→Fire, `move_speed_mult 1.3`, 3s) and
  `ice_barrier.status.ron` (Fire→Ice, `damage_taken_mult 0.6`, 3s — a damage-reduction stand-in for
  the deferred next-hit absorb).
- **Debug hotkey (M).** `hero/systems/debug.rs::debug_swap_to_mage` (`#[cfg(debug_assertions)]`,
  mirrors `debug_force_level_up`) re-identifies the live player as the Mage for Windows
  playtesting — no character-select screen yet. The campaign bot never presses M, so the baseline
  is untouched.
- **Presentation pass (§8.5, pure presentation — never runs headless).** `projectile/systems/
  visuals.rs::attach_projectile_visuals` (`Added<ProjectileMotion>`) dresses travelling projectiles
  (previously invisible) with an element-tinted circle; `status/systems/visuals.rs::
  tint_status_effects` recolors enemies by their active status (frostbite blue, blaze orange, bleed
  red, root/stun yellow). Registered in `PresentationPlugin` only, so headless tests and the
  baseline are unaffected. **Deferred:** the Blood Boil nova flash — the cone-flash path is
  logic-side, so a nova flash the same way would spawn entities in the DK campaign and move the
  baseline; it needs a presentation-only cast-VFX bus (§8.6).
- **Deferred with revival triggers** (phase4-plan §7 / architecture-plan §8.6): frost-charge class
  resource + UI bar, Frost Impale + `channel_while_moving`, dash/movement ability, real absorb
  shields, code-driven status/ability hooks + the `execute_ready_abilities` split (no hook landed
  this slice), `Override(0)` cooldown semantics, per-hero base-stat application, character-select
  UI, and full Mage progression content.
- **Sim helpers.** `set_hero(entity, id, stance)`, `hero_id()`, `active_stance()`, `tap_mouse()`;
  `assets_loaded` extended to await `HeroLibrary` + the deferred grant.
- **Tests: 84 passing** (was 73): +5 unit (`DefLibrary::get`; `HeroDef` parse ×2 across DK/Mage;
  `resolve_slot` ×2) and +6 golden scenarios (`tests/hero_stance.rs`: DK LMB regression, Mage basic
  through input slots, stance-swap-remaps-LMB, swap-effect applied, non-stance Q no-op, debug
  hotkey). Build remains warning-free.

### Phase 5 — Enemy Ability System + AI + Faction-Aware Engine (complete)
Enemies become data-driven and cast through the **same** ability engine as the player. The engine
is now faction-aware (an enemy's cast hits the player; a player's cast hits enemies), contact melee
is a first-class auto-cast ability, a ranged caster fires projectiles at the player, enemies carry a
data-only scaling model, and the long-parked `suppress_abilities` (stun) debt is wired. Delivered in
five compat-gated steps (5A–5E). **The golden baseline did NOT move at any step** — the faction
refactor preserves target sets/order, the contact-melee-as-ability change reproduces the prototype's
exact cadence (verified byte-identical, no regeneration), and every other addition is inert in the
Death-Knight campaign. See `docs/phase5-plan.md` for the plan + as-built notes.

- **Faction-aware engine (5A, neutral).** New `core::components::Faction { Friendly, Hostile }`
  (+ `opposing()`); the player is `Friendly`, enemies `Hostile`. `execute_ready_abilities` gathers
  candidates once per faction and hands each cast the list **opposing** its caster
  (`AbilityContext.enemies` → `targets`; `EnemyTarget` → `Target`). `ProjectilePayload` gained a
  `target_faction` (baked at spawn as the opposite of the caster's faction); `projectile_collision`
  collides only that faction. A player's Frostbolt still hits the same enemy set in the same order
  (baseline-neutral); an enemy's bolt now hits the Friendly player (the player's `Hurtbox`, added in
  Phase 3.1 "ready for Phase 5 enemy shots", is finally used).
- **`EnemyDef` data-drive (5B, neutral).** `EnemyDef` is now a live `DefAsset` (`.enemy.ron`, via
  `register_def_library::<EnemyDef>()`), the single source of truth per enemy — stats, appearance
  (shape + rgb → `EnemyAppearance`), `spawn_weight`, `ai_behavior`, `preferred_range`,
  `abilities: Vec<AbilityId>`, `xp_value`, and a `scaling` curve. The compiled
  `enemy/archetypes.rs` (`EnemyArchetype`/`archetypes()`/`pick()`) is **deleted**; Grunt/Runner/Brute
  are ported to `assets/enemies/{grunt,runner,brute}.enemy.ron` with **byte-identical** logic
  numbers. `enemy_bundle`/`spawn_enemy_from_def` build the enemy plus one `AbilityInstance` per
  declared ability. The ambient `spawn_enemy_over_time` weighted-picks a loaded `EnemyDef` (still
  `thread_rng`, still paused in scenarios). (`EnemyDef`/`ThemeDef`/`enemy/behavior.rs` were
  uncompiled scaffolds — never declared in `mod.rs`; `behavior.rs`'s `AiBehaviorRegistry` is deleted,
  see AI below. `ThemeDef` stays scaffold-only until Phase 7.)
- **Contact melee is a first-class ability (5B, neutral).** The hardcoded `enemy_attack` system and
  the `AttackStats`/`AttackCooldown` components are **removed**; each enemy carries an auto-cast
  `*_contact` ability (`grunt_contact` 5/28/1.0s, `runner_contact` 3/24/0.7s, `brute_contact`
  12/32/1.6s) with a new `contact_melee` behavior (hits opposing-faction actors within `range`, no
  aim, damage via the ability's `effects`). Cadence fidelity — first-hit-on-contact + no wasted swing
  out of range — is preserved by (a) spawning the ability instance **with** the enemy (no
  `Added<Enemy>` race), and (b) a new `AbilityBehavior::consumes_cooldown_on_whiff()` (default
  `true`; `contact_melee` returns `false`) so `execute_ready_abilities` skips the cooldown reset when
  a gated behavior resolves nothing. `melee_cone`/`self_nova`/`projectile` keep the default, so
  Death Strike / Blood Boil / Frostbolt are unchanged. Net: the grunt/brute contact damage now flows
  from `execute` instead of `enemy_attack`, but lands on identical frames for identical amounts —
  **golden campaign byte-identical, no regeneration.**
- **AI dispatch = component enum (5B, neutral).** New `enemy::components::AiBehavior`
  (`MeleeChaser | RangedCaster{preferred_range} | Stationary`), set at spawn from
  `EnemyDef.ai_behavior`. `enemy_follow_flow_field` + `update_enemy_facing` are gated to
  `MeleeChaser` (all ported enemies are chasers ⇒ neutral). This deliberately supersedes the scaffold's
  `&mut World`-free `AiBehaviorRegistry`/`EnemyAiHook` (which could not express flow-field steering):
  movement AI needs world access; content-extensibility is already served by the ability
  `BehaviorRegistry`. A new AI = one variant + one system (mirrors Phase 3 replacing the hook-first
  status sketch with a declarative model).
- **Ranged caster (5C, neutral to the master).** New enemy `spitter` (`ranged_caster`,
  `preferred_range: 140`, ability `spitter_bolt` — an auto-cast `projectile`). `ranged_caster_ai`
  (in `MovementSet::Intent`) approaches via the flow field until within `preferred_range`, then stops
  and **faces the player** (independent of velocity, so the aim-dependent projectile can fire while
  standing still). Its bolt bakes the Friendly target faction and hits the player, passing through
  other Hostiles. The spitter is deliberately **not** added to the golden campaign (covered by
  scenarios), so the master is untouched.
- **Enemy scaling — data-only model (5D, neutral at depth 0).** `EnemyDef.scaling: EnemyScaling`
  (`health_/damage_/xp_per_depth`, additive per depth) + a pure `resolve_enemy_stats(def, depth)`
  resolver. Health/xp scale at spawn; damage is delivered by a generic
  `core::components::DamageDealtModifier` (mirror of `DamageTakenModifier`, read on the
  `DamageEvent.source` by `apply_damage`) inserted only when depth > 0. There is **no live driver**
  yet — every shipped spawn passes `depth = 0` ⇒ base stats and no modifier ⇒ byte-identical. A
  balance sweep (or `Sim::spawn_enemy_at_depth`) can spawn at depth > 0. Resolves architecture-plan
  §8.1(7) as "scaling in data."
- **`suppress_abilities` wired (5D, neutral — §8.5 debt paid).** New marker
  `core::components::AbilitiesSuppressed`, reconciled by `resolve_actor_status` exactly like
  `Immobilized`. A suppressed caster is skipped by `auto_cast_abilities`, `execute_ready_abilities`,
  and the hero `resolve_input_to_ability` / `handle_stance_swap` (a stunned player or enemy cannot
  cast, auto-cast, or stance-swap). No shipped content applies `stun` and the campaign never stuns,
  so the marker is never present ⇒ baseline unchanged; reachable via `Sim::apply_status(.., "stun")`.
- **Presentation.** `draw_enemy_attack_flash` (a debug gizmo, presentation-only) was repointed from
  the removed `AttackCooldown` to the enemy's contact `AbilityCooldown` — flashes when a fired
  ability's cooldown is fresh. `EnemyShape` moved to `enemy/components.rs` (gained `Deserialize`).
- **Sim helpers.** `spawn_enemy(id, tile)` / `spawn_grunt` now spawn by `EnemyDef` id;
  `spawn_enemy_at_depth(id, tile, depth)`; `enemy_ability_ids(entity)`; `faction(entity)`;
  `assets_loaded` awaits `EnemyLibrary`. The three `spawn_enemy(&archetypes()[2], …)` brute call
  sites (combat/progression/golden campaign) became `spawn_enemy("brute", …)`.
- **Tests: 94 passing** (was 84). +3 unit (`EnemyDef` parse ×2, `resolve_enemy_stats` scaling math)
  and +7 golden scenarios (`tests/enemy.rs`: declared-stats spawn, contact hits the player not other
  enemies, player casts don't self-hit, ranged caster stops-and-shoots, enemy bolt through a Hostile,
  scaling health+damage by depth, suppressed caster can't cast). `tests/combat.rs::grunt_contact_
  attack_cadence` unchanged (contact cadence via the ability path). Build warning-free; golden
  baseline unchanged (no regeneration this phase).

### Phase 6 — Persistent Zones + Code-Driven Ability Hooks (complete)
Brings the `zone` module online (added to `lib.rs`/`GameLogicPlugin`) and delivers persistent ground
zones end-to-end: queryable presence, occupant DoT/regen, and AMZ projectile blocking — plus the
long-deferred **code-driven ability-hook system** (the §8.5 `execute_ready_abilities` split), whose
first consumer is the zone-conditioned Blood Boil talent. Two owner decisions widened the phase
beyond architecture-plan §7's three bullets (see `docs/phase6-plan.md`): **D1** build the real hook
registry (not a declarative shortcut), **D2** full scope incl. occupant ticks + AMZ. Delivered in
six compat-gated steps (6A–6F). **The golden baseline did NOT move at any step** — every addition is
inert in the Death-Knight campaign (no zone is ever cast; the validation talent is held out of the
offerable pool; AMZ zones touch no snapshot column and the campaign has no enemy projectiles).

- **Zone core (6A, neutral).** `pub mod zone;` (was an uncompiled scaffold); `ZonePlugin` inserts
  `PlayerZonePresence` and runs the pre-written `tick_zone_lifetimes` → `move_anchored_zones` →
  `build_player_zone_presence` chain at the **end of `MovementSet::Integrate`** (after
  `world_to_grid`), so positions are settled and presence is fresh before `CombatSet::Damage` reads
  it. Respects the Phase-3.1 movement pin (zone systems never write an actor `WorldPosition`); with
  zero zones alive every system is an empty-loop no-op.
- **`dropped_zone` behavior + `ZoneSpec` schema (6B, neutral).** New `AbilityDef.zone:
  Option<ZoneSpec>` (`zone_type` + `anchor: ZoneAnchorKind{Fixed|FollowCaster}` +
  `blocks_projectiles`); `#[serde(default)]` so every non-zone ability parses unchanged. The
  `dropped_zone` behavior (`needs_aim() == false`) returns a `CastOutcome.zone` request and
  `execute_ready_abilities` builds the `PersistentZone` entity from the spec + resolved params
  (`zone_radius`/`zone_duration`) + the **caster's `Faction`** — mirroring the existing projectile
  spawn path. **D&D** (`dnd`, the BDK L1 RMB Special — stays `activation: Input`) now drops a fixed
  `death_and_decay` zone; its `damage_per_second` was set to **0** (per Mechanics it is a *buff*
  zone, not a damage zone — the `2.0` scaffold value dropped). **Tree Conduit** (`tree_conduit`)
  ships as a marker-only zone demonstrator (Druid enhanced-attack consumer deferred). D&D is `Input`
  and the campaign bot never fires it ⇒ no zone spawns in the master ⇒ neutral.
- **Code-driven ability hooks + execute split (6C, neutral — §8.5 debt paid).** New `ability/hooks.rs`:
  `AbilityHook` trait (`pre(&mut ResolvedParams)` / `post(&CastOutcome)`, both defaulting to no-ops),
  `HookContext { caster, zones }`, and a `HookRegistry` resource (mirrors `BehaviorRegistry`).
  `execute_ready_abilities` was split to interleave hooks at two points: **Pre** (resolve → behavior
  boundary; may rewrite resolved params) and **Post** (after effects apply; reads the outcome). A
  hook runs only when its `HookId` is BOTH in the caster's `ActiveHooks` (installed on talent
  acquisition since Phase 2, never consumed until now) AND registered — an un-acquired or
  not-yet-implemented hook is zero-cost and silently skipped (unlike a missing *behavior*, which
  warns). `ResolvedParams` gained `set`/`scale`. Registered: **`blood_boil_dnd_range`** (Pre).
  `bone_shield_on_kill` stays **unregistered → inert** (its shield/absorb system is deferred,
  §8.1(5)); `death_strike`'s Post-hook listing rides along harmlessly. **Split verified
  byte-identical:** no registered hook is active on any campaign-cast ability, so the refactor
  preserves the exact resolve→behavior→effects→cooldown order.
- **Validation talent — Blood Boil range inside D&D (6C, testing.md DoD).** `BloodBoilDndRange`
  (a Pre hook) doubles Blood Boil's `radius` param while the caster stands in `death_and_decay`
  (Mechanics: "Blood boil has double range when cast inside D&D") — architecture-plan §4's "Talent 3
  — Zone-interaction" realized: no D&D or base Blood Boil code touched. `blood_boil.ability.ron`
  gained `hooks: [(Pre, "blood_boil_dnd_range")]`; the existing `blood_boil_dnd_range_rare.talent.ron`
  (a `Behavior` effect) is unchanged and kept **out of `blood_boil.talent_pool`** so the campaign
  can't offer/acquire it (master stays neutral) — validated by `tests/zone.rs` instead.
- **Zone occupant-tick effects (6D, neutral).** New `ZoneEffects { damage_per_second, regen_fraction,
  tick }` (a fixed 1 Hz repeating timer — discrete ticks, no per-frame float drift, no RNG),
  attached at spawn only when the ability defines any. `zone_tick_effects` (in `CombatSet::Damage`)
  emits, per tick, a **Holy DoT** to every **opposing-faction** actor inside (Consecrated Ground) and
  **regen** to the owner while it stands inside (D&D heals `regen_percent_per_second`% of max health).
  Damage/heal flow through the shared `apply_damage`/`apply_heal`. `consecrated_ground.ability.ron`
  (AutoCast, Holy DoT) ships as a demonstrator (no Paladin hero). Neutral: no zone exists in the
  campaign. **Guard:** `execute_ready_abilities`'s candidate query and `zone_tick_effects` both filter
  `Without<PersistentZone>` — zones carry `WorldPosition` + `Faction`, so without this a friendly
  zone could be gathered as an enemy cast's target (neutral in the campaign, but correct).
- **AMZ projectile blocking (6E, neutral to the master).** New marker `ZoneBlocksProjectiles` +
  `block_projectiles_in_zones` (in `CombatSet::Damage`, ordered `after(move_projectiles).before(
  projectile_collision)` so a blocked shot never lands). A blocking zone destroys any projectile
  whose `target_faction == zone.Faction` while it is inside the zone, **unless its `source` stands
  inside** the zone (Mechanics: "no effect if enemies emit projectiles from inside — it acts as a
  barrier"). `amz.ability.ron` (BDK band-4/6, `activation: AutoCast`, `blocks_projectiles: true`)
  joins the manifest. The `FollowCaster` anchor mechanism (`move_anchored_zones`) is built + tested;
  the AMZ-epic talent that flips base AMZ to follow is deferred content. **Measured gate:** the
  master stayed **byte-identical** — AMZ zones are in no snapshot column and the campaign has no
  enemy projectiles, so even if the fixed seed unlocks `amz` nothing observable moves.
- **Sim helpers.** `zone_count`/`zone_types`/`zone_center`/`player_in_zone` (reads
  `PlayerZonePresence`); `spawn_zone(type, center, radius, duration, follow, faction)` (direct
  test spawn, for the follow-anchor mechanism); `grant_talent(id)` (installs a talent + its
  `ActiveHook` via the real `TalentAcquiredEvent` path).
- **Tests: 107 passing** (was 94). +5 unit (zone RON parse ×4 — `tree_conduit`/`consecrated_ground`/
  `amz`/no-zone; the `blood_boil_dnd_range` hook doubling `radius` only inside D&D) and +8 golden
  scenarios (`tests/zone.rs`: D&D drops a zone that expires; presence enter/exit; **D&D doubles Blood
  Boil range inside it**; Consecrated Ground DoT hits enemies inside not outside and never the
  Friendly owner; D&D regen heals the owner inside only; AMZ blocks an enemy bolt; a bolt emitted
  from inside the AMZ is not blocked; a follow-anchor zone tracks the owner). Build warning-free;
  golden baseline unchanged (no regeneration).

### Environment
- Installed Rust 1.96.1 + Cargo via rustup in WSL.
- Installed Bevy Linux system dependencies (`build-essential`, `libudev-dev`,
  `libasound2-dev`, `libwayland-dev`, `libxkbcommon-dev`).
- `cargo check` / `cargo build` pass; remaining 3 warnings are dead-code for not-yet-consumed
  Phase 3+ scaffolding (`DamageTag`/`tags`, `StatusSet`).
- Runtime note: game code is verified up to `cargo build`/`check` in WSL. `cargo run` under WSLg
  can't create a GPU device — wgpu falls back to the GL backend (Mesa/D3D12), which lacks the
  compute-shader features Bevy needs, panicking with `RequestDeviceError Device(Lost)` before any
  `Startup` system. Unrelated to Phase 0 (affects the prototype too). Tested on Windows for now;
  running under WSL is backlogged.

---

## [Prototype] — Proof of Concept
_Commits `20e87d2` → `af5126d`_

This phase proved the core gameplay loop — movement, attacks, enemies, health/damage,
pickups, and a procedural play area. The data model was intentionally kept simple;
it is being replaced in the architecture rewrite above.

### Movement & Camera (`20e87d2`, `7a092c2`)
- WASD player movement with `WorldPosition` + `Velocity` component pair.
- Per-axis wall-collision: X and Y steps tested independently against `TileMap` so the
  player slides along walls instead of stopping dead.
- Camera follow with smooth lerp.
- Mouse cursor tracked and unprojected to world space; stored as `Facing` direction on
  the player entity.

### Enemy Spawning & Pathfinding (`885a70e`, `badeea8`, `f452eb6`, `94ca044`)
- BFS flow field built each frame from the player's `GridPosition` outward to
  `FLOW_RADIUS` tiles, storing the direction-to-player vector at each cell.
- Enemies read their grid cell's flow direction and lerp velocity toward it — handles
  wall avoidance automatically.
- Diagonal movement supported; corner-cutting through two adjacent walls blocked.
- Timed spawner places enemies at random angles on a radius ring, skipping blocked tiles.
- Three weighted archetypes: Grunt (balanced), Runner (fast/fragile), Brute (slow/tanky).

### Attacks & Damage (`c8c7e56`, `68f709b`, `f67bc2f`, `d8d83cd`, `3b332b3`)
- Two prototype attack shapes: radial circle (Space) and melee cone (V), both
  aimed at the mouse cursor via the `Facing` component.
- Damage resolved instantly against all enemies in range the same frame the key is pressed.
- Transient VFX entity spawned per attack so gizmos can draw the hitbox shape.
- `DamageEvent` → `apply_damage` → `Health` chain introduced; single consumer pattern
  established. `LastHitBy` component tracks the damage source for kill credit.
- `CombatSet { Damage, Apply, Death }` system-set ordering ensures a hit fully resolves
  within one frame.

### Enemy Combat (`c472cfe`, `f9e92d3`)
- Contact attack: while within `AttackStats.range` of the player and cooldown ready,
  enemy emits `DamageEvent` at the player.
- `AttackCooldown` starts elapsed (first hit lands immediately on contact).
- Enemy facing updated from velocity direction each frame.

### Health, Death & XP (`27243aa`, `aba708a`, `6d6eb9a`)
- `Health { current, max }` shared between player and enemies.
- `player_death`: despawns the player entity when health ≤ 0 (placeholder — will become
  a `GameState::GameOver` transition).
- `enemy_death`: despawns enemy, emits `GainXpEvent` to the killer (`LastHitBy`).
- `Experience` component with linear XP curve (`XP_FIRST_LEVEL + (level-1) * XP_LEVEL_STEP`).
  Level-up overflow carries into the next level. `LevelUpEvent` fired per level gained.
  Level-up reward is currently a log line — hook for the talent system.

### Pickups (`8493885`)
- `PickUp` entity with `PickUpKind::Heal(f32)` payload.
- Proximity collection: player walks over pickup → `HealEvent` emitted → `apply_heal`
  clamps to `Health.max`.
- Enemies have a random drop chance for a heal pack on death.
- Timed ambient spawner drops pickups near the player.

### Procedural World (`af5126d`)
- `TileMap` — sparse `HashSet<GridPosition>` of blocked tiles; out-of-bounds treated as
  impassable. `is_blocked` / `in_bounds` used by movement, flow field, and spawner.
- `generate_map`: solid border ring + random-walk obstacle blobs scattered across the
  interior. Spawn-clear box around the origin always left walkable.
- Map rendered as colored quads per tile via a startup system.

---

## What Was Not Built (intentional scope boundary)

Phases 0–6 (foundation, ability system, talent system, status effects, hero/stance system + Mage,
enemy abilities + AI + faction-aware engine, persistent zones + code-driven hooks) are complete. The
following are designed and scaffolded but have zero implementation yet:

- ~~Hero / stance system (HeroDef asset, Q swap) — Phase 4~~ **done** (focused vertical slice —
  Death Knight + Mage; heavier Mage subsystems deferred, see architecture-plan §8.6)
- ~~Enemy ability kits and AI registry — Phase 5~~ **done** (data-driven `EnemyDef`, faction-aware
  ability engine, contact melee + a ranged caster, data-only scaling, `suppress_abilities` wired;
  the AI "registry" became a component enum — see architecture-plan §8.7. Deferred: `ThemeDef`/theme
  spawning + `Elite`/boss spawn roles + boss AI + a live scaling driver — Phase 7/9)
- ~~Persistent zones (D&D, Consecrated Ground, Tree Conduit) — Phase 6~~ **done** (full scope:
  `dropped_zone` + `PlayerZonePresence` + occupant DoT/regen + AMZ projectile blocking; plus the
  code-driven ability-hook system — `HookRegistry`/`AbilityHook` + the `execute_ready_abilities`
  split — validated by Blood Boil's double-range-inside-D&D talent. Deferred to Phase 9 class
  content: cross-ability zone buffs, Tree Conduit's enhanced-attack consumer, the AMZ-follow talent,
  and the bone-shield Post hook — see architecture-plan §8.8)
- Act graph, room types, encounter lifecycle — Phase 7
- Persistence (save/load RunState, MetaState) — Phase 8
- Full class content (Druid, Paladin, Mage capstones; all enemies and bosses) — Phase 9
- All UI beyond a debug health bar gizmo + talent picker (no HUD, character select, or menus)
