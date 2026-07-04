# Changelog

---

## [Unreleased] — Architecture, Scaffold, Phases 0–2 & Testing Infrastructure
_2026-07-04/05 — commits `5067dfb` (scaffold) → `2963e56` (phase 0) → `894452d` (phase 1) → `87b24ae` (docs) → `bc9d1d2` (phase 2) → testing infrastructure (stages 0–2)_

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

Phases 0–2 (foundation, ability system, talent system) are complete. The following are designed
and scaffolded but have zero implementation yet:

- Status effects (bleed, blaze, frostbite, etc.) — Phase 3
- Hero / stance system (HeroDef asset, Q swap) — Phase 4
- Enemy ability kits and AI registry — Phase 5
- Persistent zones (D&D, Consecrated Ground, Tree Conduit) — Phase 6
- Act graph, room types, encounter lifecycle — Phase 7
- Persistence (save/load RunState, MetaState) — Phase 8
- Full class content (Druid, Mage, Paladin; all enemies and bosses) — Phase 9
- All UI beyond a debug health bar gizmo
