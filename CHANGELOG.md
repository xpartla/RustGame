# Changelog

---

## [Unreleased] — Architecture, Scaffold & Phases 0–1
_2026-07-04 — commits `5067dfb` (scaffold) → `2963e56` (phase 0) → `894452d` (phase 1) → `87b24ae` (docs)_

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

Phase 0 (foundation) and Phase 1 (ability system) are complete. The following are designed and
scaffolded but have zero implementation yet:

- Talent system (modifier stack, offer generation) — Phase 2
- Status effects (bleed, blaze, frostbite, etc.) — Phase 3
- Hero / stance system (HeroDef asset, Q swap) — Phase 4
- Enemy ability kits and AI registry — Phase 5
- Persistent zones (D&D, Consecrated Ground, Tree Conduit) — Phase 6
- Act graph, room types, encounter lifecycle — Phase 7
- Persistence (save/load RunState, MetaState) — Phase 8
- Full class content (Druid, Mage, Paladin; all enemies and bosses) — Phase 9
- All UI beyond a debug health bar gizmo
