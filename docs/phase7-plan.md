# Phase 7 Implementation Plan — Act Graph + Room / Encounter System

_Written 2026-07-05 against `main` after Phase 6 (persistent zones + hooks). Companion to
`docs/architecture-plan.md` (§3.8 run graph, §3.9 enemy/AI + themes, §3.10 persistence scopes,
§3.11 seeded RNG, §6 Q1 ThroneRoom, §7 phase plan, §8.1(7)/(9)/(10) amendments) and
`docs/testing.md`. This is the **largest structural phase** — it turns the single flat arena into a
seeded, branching, themed act of typed encounters. Implement it in the compat-gated sub-steps of §5.
As-built notes go in §9._

> **For the implementing agent:** read `docs/phase6-plan.md` first for the house style (compat-gate
> after every step; the golden master is the contract; declare any baseline move in the CHANGELOG).
> The §0 decisions below are **proposed defaults — confirm them with the project owner before 7A**,
> exactly as Phases 4/5/6 locked their decisions up front. Everything else is specified concretely.

---

## 0. Proposed decisions (confirm with the owner before implementing)

| # | Decision | Recommendation & rationale |
|---|---|---|
| **D1 — Golden-master strategy** | Does Phase 7 keep the golden campaign **byte-identical**, or regenerate it as a declared change (the game now boots into a real encounter)? | **Keep it byte-identical (recommended).** Make **auto-start-run a windowed-only Startup system** (added by `GamePlugin`, like `PresentationPlugin` — NOT by `GameLogicPlugin`). The headless sim never auto-starts a run; encounter scenarios opt in via a new `Sim::start_run(seed)`. The encounter *systems* live in `GameLogicPlugin` but are gated on a `CurrentEncounter` resource, so with no run active (the golden campaign's world) they are inert. Result: the campaign + every existing scenario stay unchanged. This matches the project's discipline (Phases 4–6 all landed baseline-neutral). |
| **D2 — Scope** | Full (graph + rooms + encounter lifecycle + objectives + scaling driver + spawn roles + **ThroneRoom** + **Merchant node**) in one phase, or a focused core with ThroneRoom/Merchant as a follow-up? | **Full, but sequenced (recommended).** The core lifecycle (7A–7E) ships first and is independently useful; ThroneRoom + Merchant land in 7F as a clearly separated, compat-gated step. If the owner wants to bound risk, cut 7F to a "Phase 7.5". |
| **D3 — Node selection** | How does the player choose the next node with no map-view UI yet (UI is its own deferred phase, §8.1(9))? | **A minimal keyboard picker on a new `GameState::MapSelect` overlay (recommended)** — mirrors the Phase-2 `TalentPicker` (press `1/2/3` to pick among reachable branches; sim-drivable headless). The full visual act-graph map view is deferred to the UI phase. Fallback: deterministic auto-advance (pick the lowest-id branch) if even the minimal picker is out of scope. |
| **D4 — Theme enemy content** | The 5 designed themes reference ~25 enemies + bosses that **don't exist** (only `grunt`/`runner`/`brute`/`spitter` do). Content is Phase 9. | **Ship the `ThemeDef` loader + all 5 theme files, but point their pools at EXISTING enemies for now (recommended)**, plus **one placeholder boss enemy def** (`warlord` — a beefed-up melee/caster) so `BossRoom`/`KillMapBoss`/`ActBoss` actually function and are testable. Phase 9 swaps the rosters to the real per-theme content (a pure data edit — no code). Rejected: shipping the designed names (scorpion, …) that fail to load — encounters would spawn nothing and can't be tested. |
| **D5 — Depth / scaling driver** | What feeds `depth` into `resolve_enemy_stats(def, depth)` (the Phase-5 data-only scaling)? | **`depth = (current_act − 1) × COLUMNS_PER_ACT + node_column` (recommended)** — a monotonic "how deep into the run" index, tunable. At the Act-1 tutorial (act 1, column 0) `depth = 0` ⇒ base stats (neutral, matches Phase 5's promise). |

---

## 1. Scope

### In scope
1. **`ThemeDef` loader.** Add `serde::Deserialize` + `impl DefAsset` (`.theme.ron`) to the existing
   `ThemeDef` (enemy/assets.rs) and register via `register_def_library::<ThemeDef>()`. Ship 5 theme
   files (D4: pools point at existing enemies for now).
2. **Act graph generation.** Compile `world/graph.rs` (currently an uncompiled scaffold —
   `ActGraph`/`EncounterNode`/`EncounterType`/`ObjectiveType`/`RoomModifierDef` already defined) and
   add a **pure, seeded** `build_act_graph(act, rng) -> ActGraph` (Slay-the-Spire columns).
3. **Per-encounter room generation.** Compile `world/generator.rs` (scaffold `generate_room`
   dispatch with `todo!()` sub-generators). Port the existing `generate_map` blob into
   `procedural_room_layout`; add boss / throne / act-boss / merchant layouts.
4. **Run lifecycle.** Enable `run/{state,plugin,systems}` (currently commented out in `run/mod.rs`).
   `RunState` + `RunRng` become **in-memory** resources (serialization is Phase 8); `RunPlugin` joins
   `GameLogicPlugin`; a windowed-only auto-start (D1).
5. **Encounter lifecycle.** Load an encounter (generate room → spawn themed enemies at the node's
   depth) → track its objective → on completion enter `MapSelect` → player picks a reachable node →
   tear down → load the next. Objectives: `KillAll`, `Survive{secs}`, `KillMapBoss{boss_id}`.
6. **Enemy spawning + scaling + roles.** A **seeded** (RunRng) encounter spawner replacing the
   ambient `spawn_enemy_over_time`; the **live scaling driver** (D5) feeding
   `spawn_enemy_from_def(…, depth)`; `EnemyRarity` spawn roles (Common pack vs. `MapBoss`/`ActBoss`
   from the theme's boss pools).
7. **ThroneRoom** (7F): distinct layout + a mandatory **curse** (`RoomModifierDef` → threaded into
   `resolve_params`'s `extra_modifiers`, which already exists) + a **kiss** reward (reuse the
   already-registered `ThroneRoomRewardEvent` → `TalentPicker` with a Rare-or-above floor).
8. **Merchant node** (7F): a traversable no-combat rest node (its remove/trade **ops** are deferred).
9. **Tutorial map**: Act-1 entry is a fixed `Map` node calibrated so the player reaches ~level 2.
10. **Full test suite + docs.**

### Out of scope (explicitly deferred — see §7)
Full visual act-graph **map UI** + HUD (the UI phase, §8.1(9) — Phase 7 ships only the minimal
keyboard picker); **merchant ops** (remove / 3-for-1 trade — Phase 8/9); **RunState serialization /
resume** + `SmallRng` serde + score computation (Phase 8, §8.2); the **real per-theme enemy rosters**
(25+ enemies + multi-ability bosses — Phase 9 content); **multi-phase boss AI** (Phase 9); the
**Act-3 secret level** (later); character-select (Phase 8).

---

## 2. Architecture

### 2.1 `ThemeDef` loader (mirrors the Phase-4 HeroDef / Phase-5 EnemyDef pattern)

`ThemeDef` (enemy/assets.rs) already has the fields but lacks `Deserialize` + `DefAsset`. Add both:

```rust
#[derive(Asset, TypePath, Debug, Clone, serde::Deserialize)]   // + Deserialize
pub struct ThemeDef { id, display_name, common_enemy_pool, boss_pool, map_boss_pool }

pub type ThemeLibrary = DefLibrary<ThemeDef>;
impl DefAsset for ThemeDef {
    const EXTENSIONS: &'static [&'static str] = &["theme.ron"];
    const MANIFEST: &'static [(&'static str, &'static str)] = &[ /* 5 themes */ ];
}
```

Register in `EnemyPlugin` (or a small `WorldPlugin` addition) with `register_def_library::<ThemeDef>()`.
Rename `assets/themes/sand_dune.ron` → `.theme.ron` and add the other four. **D4:** their pools list
existing enemy ids for now. Unit-test parse (like `EnemyDef`).

### 2.2 Act graph generation (compile `world/graph.rs` + a pure builder)

Add `pub mod graph; pub mod generator;` to `world/mod.rs`. The scaffold types are ready; add:

```rust
// world/graph.rs
pub const COLUMNS_PER_ACT: usize = 15;
/// Seed-deterministic (RunRng only). Slay-the-Spire columns: each column 1..=3 nodes; every node
/// links to 1..=2 nodes in the next column; the graph is connected front-to-back.
pub fn build_act_graph(act: u8, theme: ThemeId, rng: &mut RunRng) -> ActGraph { … }
```

Concrete generator (keep it simple + deterministic):
- **Columns** `0..COLUMNS_PER_ACT`. Column 0: a single entry node (Act 1 → the **tutorial `Map`**;
  Acts 2/3 → a normal `Map`). Last column: a single **`ActBoss`**. Second-to-last: a **`BossRoom`**.
- Middle columns: 1–3 nodes each (RunRng), typed by a weighted roll — mostly `Map` (with a RunRng
  `ObjectiveType`), with an occasional `Merchant` and one **`ThroneRoom`** guaranteed per act.
- **Edges**: for each node in column *c*, connect to 1–2 nodes in column *c+1* (RunRng), then a
  connectivity pass guarantees every next-column node has ≥1 incoming edge and every node has ≥1
  outgoing (no dead ends). `ActGraph::next_nodes` (already implemented) reads these.
- **Theme**: one theme per act (D5-adjacent), assigned by RunRng at act start; each `Map`/`BossRoom`
  node carries `Some(theme)`; `Merchant`/`ActBoss` carry `None` (matches the scaffold comment).
- **ThroneRoom modifier**: assigned from the `room_modifiers/` pool (the 3 existing curse RONs) via
  RunRng at generation time (the node's `modifier: Some(id)`).

`build_act_graph` is **pure over `&mut RunRng`** → unit-testable for determinism (same seed ⇒ same
graph) and structural invariants (connected, one ActBoss last, one ThroneRoom, N columns).

### 2.3 Per-encounter room generation (compile `world/generator.rs`)

`generate_room(encounter, map, rng)` dispatch exists; fill the `todo!()` sub-generators:
- `procedural_room_layout` — **port the existing `generate_map` blob** (border ring + random-walk
  obstacle blobs from RunRng + spawn-clear box). `generate_map` (world/systems/generate_map.rs) is
  then either deleted or reduced to a thin caller; its constants (`MAP_HALF_TILES` etc.) stay.
- `boss_room_layout` / `act_boss_layout` — border ring, few/no interior obstacles (open sightlines).
- `throne_room_layout` — a distinct hall + raised-dais geometry (part of the kiss/curse "see the
  threat" fantasy).
- `merchant_layout` — small safe room, no obstacles.

All take `&mut RunRng` (seed-deterministic; **no `thread_rng`**). Each clears + repopulates the shared
`TileMap` resource. The `TileMap` API (`in_bounds`/`is_blocked`, sparse `blocked` set) is unchanged.

**Presentation note:** `render_map` (PresentationPlugin) currently renders the map once at Startup.
Per-encounter regeneration needs the map re-rendered on change — a **presentation-only** follow-up
(despawn old floor/obstacle meshes, re-render on a `TileMap` change/`MapRegenerated` event). It never
runs headless, so it does not affect the golden master; specify it but it can trail the logic.

### 2.4 Run lifecycle (enable `run/{state,plugin,systems}`)

Uncomment `pub mod plugin; pub mod state; pub mod systems;` in `run/mod.rs`. `RunState` (state.rs) and
`RunRng` (rng.rs, already live) become in-memory resources — **no serde this phase** (Phase 8, §8.2):

- **`RunState`** holds `seed`, `hero_id`, `current_act`, `current_node`, `act_graph`,
  `player_health`, `player_level`, `unlocked_abilities`, `acquired_talents`, `level_flow` (the
  scaffold shape is already correct). Phase 7 keeps it authoritative for graph position + act; the
  player-mirror fields (health/level/talents) are written on encounter transitions but **resume**
  from them is Phase 8. (Simplest correct approach: keep the live player entity as the source of
  truth during a run; sync into `RunState` on transition for Phase-8 serialization.)
- **`start_run(world, seed, hero_id)`** (new, in run/systems): seed `RunRng`, pick the act theme,
  `build_act_graph(1, theme, rng)`, insert `RunState`, set `CurrentEncounter` to the entry node, and
  load it (§2.5). Called by the **windowed** auto-start (D1) and by `Sim::start_run`.
- **`RunPlugin`** (replace the `todo!()`): registers `EncounterCompleteEvent` (already defined) +
  the encounter/transition systems; does **not** insert `RunState` at build (inserted by `start_run`).

**`CurrentEncounter` resource** (new) — the small "what am I fighting right now" state that gates the
encounter systems, so they are inert when absent (the golden campaign's world):

```rust
#[derive(Resource)]
pub struct CurrentEncounter {
    pub node: NodeId,
    pub encounter: EncounterType,
    pub theme: Option<ThemeId>,
    pub depth: u32,
    pub objective: ObjectiveProgress,   // per-objective tracking (§2.5)
}
```

### 2.5 Encounter lifecycle + objectives

The core loop, driven by systems in `run/` (all `run_if(resource_exists::<CurrentEncounter>)` so they
never touch a runless world):

1. **Load** (`load_encounter`): `generate_room(encounter, map, rng)`; **spawn the roster**
   (§2.6) at the node's `depth`; teleport the player to a safe spawn; init `ObjectiveProgress`.
2. **Track** (`check_objective`, in `CombatSet::Death` after `enemy_death`):
   - `KillAll` → complete when the roster is fully spawned **and** no `Enemy` remains.
   - `Survive{secs}` → a countdown timer; complete on expiry (enemies may keep spawning in waves).
   - `KillMapBoss{boss_id}` → complete when the tagged boss `Entity` is dead (a `MapBoss` marker).
   - On completion → emit `EncounterCompleteEvent`.
3. **Advance** (`handle_encounter_complete`, replacing the `todo!()`): sync player state into
   `RunState`; if the node was an `ActBoss` → `current_act += 1`, rebuild the graph (or → GameOver on
   Act 3); else enter **`GameState::MapSelect`** (D3) with the reachable `next_nodes`.
4. **Select** (`MapSelect` overlay, minimal keyboard picker): player picks a reachable node →
   `RunState.current_node = chosen`; **tear down** the old encounter (despawn `Enemy`, `Projectile`,
   `PersistentZone`, `PickUp`, transient VFX) and remove `CurrentEncounter`; return to `InRun` and
   `load_encounter` the chosen node.

Health is **not** restored between rooms (RunState comment); the player entity persists across
encounters (only encounter-scoped entities are torn down).

### 2.6 Seeded encounter spawning + scaling driver + spawn roles

Replace the ambient `spawn_enemy_over_time` (thread_rng, §Discard in architecture §1) with a
**seeded** encounter spawner:

- **Roster**: from the node's `ThemeDef` — pack enemies weighted-picked from `common_enemy_pool` via
  **`RunRng`** (deterministic, unlike the old ambient roll); count/waves scale with `depth`. A
  `BossRoom` spawns one enemy from `boss_pool`; `KillMapBoss` one from `map_boss_pool` (tagged
  `MapBoss`); `ActBoss` a designated act boss.
- **Scaling driver (D5)**: every spawn goes through the existing
  `spawn_enemy_from_def(commands, def, grid, depth)` with the node's `depth`, so `resolve_enemy_stats`
  scales health/xp and inserts `DamageDealtModifier` (Phase-5 machinery, finally driven). Depth 0
  (Act-1 tutorial) ⇒ base stats.
- **Roles**: `EnemyRarity` (`Common`/`Elite`/`MapBoss`/`ActBoss`) selects which pool a spawn draws
  from and whether it gets a `MapBoss`/boss marker (for `KillMapBoss` tracking + future UI).
- The `EnemySpawner` ambient timer resource can stay for now but the encounter spawner is the real
  driver; the golden campaign (no run) is unaffected either way (it spawns waves manually).

### 2.7 ThroneRoom — curse + kiss (7F)

- **Curse**: the node's `RoomModifierDef` (loaded from `assets/room_modifiers/*.ron` — 3 exist:
  `no_regen`, `enemies_deal_double_damage`, `player_slowed`) contributes `curse_modifiers:
  Vec<StatModifier>`. Wire a `RoomModifiers(Vec<StatModifier>)` resource, populated on entering a
  ThroneRoom and cleared on leaving; `execute_ready_abilities` passes it as `resolve_params`'s
  `extra_modifiers` (currently always `&[]` — the hook already exists). This is the intended
  mechanism (architecture §3.8 / talent/modifier.rs). **Note:** curses that aren't ability-param
  modifiers (e.g. "no regen", "enemies deal double damage") may need their own small consumers — scope
  each `RoomModifierDef` to what it actually needs; the plan's default is the `extra_modifiers` path
  for the player-stat curses, with the others flagged for their own tiny gates.
- **Kiss**: on entering, emit the already-registered `ThroneRoomRewardEvent` → the progression flow
  opens `TalentPicker` with a **Rare-or-above** rarity floor (the offer generator already supports a
  rarity filter; `LevelUpFlowState`/`refill_offer` already scaffolds this per progression/state.rs).
- **Layout**: `throne_room_layout` (distinct geometry, §2.3).

### 2.8 Merchant node (7F)

A traversable no-combat rest node: `merchant_layout` (empty safe room), no roster, objective
auto-completes (or a "walk to exit" trigger). The **remove-talent / 3-for-1 trade ops** are deferred
(Phase 8/9) — Phase 7 just lets the player pass through. (`GameState::Merchant` exists; a real
merchant overlay is the UI phase.)

### 2.9 GameState + frame integration

- Add (or use) **`GameState::MapSelect`** for node selection (freezes the world like `TalentPicker`,
  since gameplay is `InRun`-gated). Add to the `GameState` enum (a reserved-variant addition).
- No new system *sets* — encounter systems slot into the existing chain: spawning/objective checks
  in `CombatSet::Death` region; `MapSelect` input ungated (overlay). Room generation runs on
  `load_encounter` (a one-shot, not per-frame).
- **Auto-start (D1, windowed-only):** `GamePlugin` (not `GameLogicPlugin`) adds a Startup system
  `auto_start_run` calling `start_run(seed_from_entropy, default_hero)`. The headless sim never
  auto-starts; `Sim::start_run` drives it in encounter scenarios.

### 2.10 Golden-master neutrality (the load-bearing constraint — D1)

The golden campaign uses `Sim::new_arena`, which never calls `start_run` ⇒ no `CurrentEncounter`/
`RunState` ⇒ every encounter system is `run_if`-gated off ⇒ the campaign world is exactly today's
(empty arena, scripted waves, manual spawns). The only schedule change is *adding* gated-off systems;
the Phase-3.1 movement pin keeps positions stable (verified to hold across Phases 3–6 when adding
systems). **Expected: byte-identical, no regeneration.** If any diff appears, it means an encounter
system ran without a `CurrentEncounter` (a gating bug) — fix the gate, do not regenerate.

---

## 3. File-level change map

| Area | File(s) | Change |
|---|---|---|
| Theme | `enemy/assets.rs` | `ThemeDef`: `+ Deserialize`, `impl DefAsset` (`.theme.ron`), `ThemeLibrary`, MANIFEST (5); parse tests |
| Theme | `assets/themes/*.theme.ron` | rename `sand_dune.ron`; add 4; pools → existing enemies (D4) |
| Graph | `world/mod.rs` | `pub mod graph; pub mod generator;` |
| Graph | `world/graph.rs` | `build_act_graph(act, theme, rng)` + `COLUMNS_PER_ACT`; determinism/invariant unit tests |
| Room | `world/generator.rs` | fill the 5 `todo!()` layout generators; port the blob into `procedural_room_layout` |
| Room | `world/systems/generate_map.rs` | fold into `procedural_room_layout` (delete or thin) |
| Run | `run/mod.rs` | enable `plugin`/`state`/`systems` modules |
| Run | `run/plugin.rs` | replace `todo!()` — register `EncounterCompleteEvent` + encounter systems |
| Run | `run/state.rs` | `RunState` live (in-memory; serde is Phase 8); `CurrentEncounter` + `ObjectiveProgress` (new) |
| Run | `run/systems/transitions.rs` | implement `handle_encounter_complete`; `start_run`; `load_encounter`; `check_objective`; teardown |
| Run | `run/systems/select.rs` (new) | `MapSelect` keyboard picker |
| Enemy | `enemy/systems/spawner.rs` | seeded encounter spawner (roster from ThemeDef via RunRng, depth-driven); retire/replace ambient |
| Enemy | `enemy/components.rs` | `MapBoss` marker (for `KillMapBoss`) |
| Ability | `ability/systems/execute.rs` | pass `RoomModifiers` as `resolve_params`'s `extra_modifiers` (ThroneRoom curse) |
| Curse | `world/graph.rs` (`RoomModifierDef`) + a loader | load `assets/room_modifiers/*.ron`; `RoomModifiers` resource |
| Progression | `progression/systems/offer.rs` | consume `ThroneRoomRewardEvent` → Rare-floor offer (scaffold exists) |
| State | `game/state.rs` | `+ MapSelect` variant |
| Wiring | `game/plugin.rs` | `GameLogicPlugin += RunPlugin`; `GamePlugin += auto_start_run` (windowed-only, D1) |
| Sim | `src/sim/mod.rs` | `start_run(seed)`; `current_node`/`act_graph`/`encounter` accessors; `advance_to_node`; theme/depth helpers |
| Content | `assets/enemies/warlord.enemy.ron` (new) | placeholder boss (D4) |
| Tests | `tests/act_graph.rs`, `tests/encounter.rs` (new) | determinism + lifecycle scenarios |
| Docs | this §9; CHANGELOG; architecture-plan §8.9 + §8.1 status; testing.md; Mechanics.md; repo CLAUDE.md | 7G |

---

## 4. Content

### 4.1 Themes (`*.theme.ron`, D4 — existing enemies for now)
5 files (`sand_dune`, `forest`, `castle_ruins`, `frozen_wasteland`, `alpine_lakeside`). Each
`common_enemy_pool` draws from `[grunt, runner, brute, spitter]`; `boss_pool` / `map_boss_pool` use
the new `warlord`. A `// Phase 9:` comment notes the real rosters replace these (pure data edit).

### 4.2 Placeholder boss (`warlord.enemy.ron`)
`rarity: MapBoss`, high health (e.g. 120), a real ability (reuse `brute_contact` or a new
`warlord_smash` cone), `spawn_weight: 0` (never ambient-picked; spawned only by boss roles). Enough
to make `BossRoom`/`KillMapBoss`/`ActBoss` end-to-end testable; multi-phase boss AI is Phase 9.

### 4.3 Room modifiers
The 3 existing `assets/room_modifiers/*.ron` (`no_regen`, `enemies_deal_double_damage`,
`player_slowed`) become the ThroneRoom curse pool (add `Deserialize` + a small loader; they are
plain `RoomModifierDef` structs, not yet `DefAsset`).

---

## 5. Implementation sequence (each step independently `/compat-check`-able)

Ordered so the golden master stays byte-identical throughout (D1). Confirm §0 first.

**7A — ThemeDef loader (neutral).** ★ `Deserialize` + `DefAsset` + `ThemeLibrary` + 5 theme files
(D4); register it; parse tests. No behavior wired ⇒ byte-identical. Gate: baseline byte-identical.

**7B — Act graph generation (neutral).** ★ Compile `world/graph.rs`; `build_act_graph` (pure, seeded);
determinism + invariant unit tests. Not yet driving anything ⇒ neutral.

**7C — Room generation + run scaffolding compiled (neutral).** ★ Compile `world/generator.rs` (port
the blob); enable `run/{state,plugin,systems}`; `CurrentEncounter`/`ObjectiveProgress`; `RunPlugin`
in `GameLogicPlugin` (systems gated on `CurrentEncounter`, absent ⇒ inert). `start_run`/`load_encounter`
exist but nothing calls them in the sim. Gate: byte-identical (all new systems gated off).

**7D — Encounter lifecycle end-to-end via the sim (neutral to the master).** ★ `Sim::start_run` +
encounter helpers; `load_encounter` (seeded roster + depth scaling + roles), `check_objective`
(KillAll/Survive/KillMapBoss), `handle_encounter_complete`, `MapSelect` picker, teardown. Exercised by
`tests/encounter.rs` only; the golden campaign never starts a run ⇒ **master byte-identical.**

**7E — Windowed auto-start (D1, windowed-only).** ★ `GamePlugin += auto_start_run` (headless sim
unaffected — it's not in `GameLogicPlugin`). The windowed game now boots into Act-1 encounter 1.
Gate: byte-identical (auto-start is not in any sim path). _(Verify on Windows that a real run plays.)_

**7F — ThroneRoom + Merchant (neutral to the master).** ★ ThroneRoom curse (`RoomModifiers` →
`extra_modifiers`) + kiss (`ThroneRoomRewardEvent` → Rare-floor picker) + `throne_room_layout`;
Merchant rest node. Covered by scenarios; no run in the campaign ⇒ neutral. **Caution:** the
`extra_modifiers` wiring touches `execute_ready_abilities` — with an **empty** `RoomModifiers`
(the campaign's state, since `resolve_params` is already called with `&[]`) it must be byte-identical;
verify.

**7G — Docs + final gate.** §9 as-built; CHANGELOG "Phase 7"; architecture-plan §8.9 + §8.1(7)/(9)/(10)
status + §7 marker; testing.md Phase-7 DoD delivered; Mechanics.md notes (themes/bosses are
placeholders; encounter flow live); repo CLAUDE.md. Full `/compat-check`.

> **If D2 = focused:** stop after 7E (+7G docs), and schedule 7F (ThroneRoom/Merchant) as Phase 7.5.

---

## 6. Validation & testing suite

### 6.1 Unit tests
- `ThemeDef` RON parse (5 files); `RoomModifierDef` parse.
- `build_act_graph` **determinism** (same seed ⇒ identical graph) and **invariants** (exactly
  `COLUMNS_PER_ACT` columns; single entry; single `ActBoss` in the last column; ≥1 `ThroneRoom`; fully
  connected — every node reachable from entry, no dead ends).
- Room generators: each produces a bordered, in-bounds map with a walkable spawn-clear box;
  `procedural_room_layout` reproduces the old `generate_map` signature for the same seed (a nice
  regression pin — the blob port is behavior-preserving).
- The depth formula (D5) at act/column boundaries.

### 6.2 Golden scenarios (`tests/act_graph.rs` + `tests/encounter.rs`)
1. **act_graph_is_seed_deterministic** *(testing.md Phase-7 DoD)* — two `build_act_graph` calls with
   the same seed produce identical node/edge sets; different seeds differ.
2. **graph_is_connected_with_one_act_boss** — structural invariants hold.
3. **encounter_spawns_themed_roster** — `Sim::start_run` → the entry encounter spawns enemies drawn
   from the act theme's pool, seed-deterministically (same seed ⇒ same roster).
4. **objective_completion_advances_the_node** *(testing.md Phase-7 DoD)* — clear a `KillAll` room
   (kill the roster) → `EncounterCompleteEvent` → `MapSelect` → pick a branch → the next encounter
   loads and `current_node` advanced.
5. **survive_objective_completes_on_timer**; **kill_map_boss_completes_on_boss_death** (the tagged
   `MapBoss`, not a pack kill).
6. **enemy_scaling_deepens_with_node_depth** — an enemy spawned at a deeper node has scaled
   health + a `DamageDealtModifier` (drives the Phase-5 curve via the real encounter path).
7. **throne_room_applies_curse_and_offers_reward** (7F) — entering a ThroneRoom applies its curse
   (an ability's resolved param reflects the `extra_modifiers`) and opens the Rare-floor picker.
8. **tutorial_map_is_act1_entry** — Act-1 column 0 is a `Map` at depth 0 (base stats).

New sim helpers: `start_run(seed)`, `current_encounter()`, `act_graph()`, `advance_to_node(id)` /
`pick_branch(i)`, `current_depth()`.

### 6.3 Golden master
**Byte-identical, no regeneration** (D1): the campaign never starts a run, so encounter systems stay
gated off. The reproducibility tripwire must still pass — encounter generation/spawning is **RunRng**
(seed-deterministic), no `thread_rng` in any gameplay path. If the master moves, a gate leaked → fix
the gate, don't regenerate.

### 6.4 Compat gate
`/compat-check` at every ★. Every step expects **no** master diff. Any diff, or any drift, is a
regression (a gating bug — an encounter system running without `CurrentEncounter`).

---

## 7. Deferred — with the trigger that revives each

| Deferred | Revived by |
|---|---|
| Real per-theme enemy rosters (25+ enemies, multi-ability bosses) | Phase 9 content (a data edit — swap theme pools + add `.enemy.ron`) |
| Multi-phase boss AI (act bosses, boss rooms) | Phase 9 boss design |
| Visual act-graph **map view** + HUD (health/cooldowns/XP/objective) | the UI phase (§8.1(9)) — Phase 7 ships only the keyboard picker |
| Merchant **ops** (remove talent / 3-for-1 trade) | Phase 8/9 (needs the merchant overlay + `TalentRemovedEvent` path, scaffolded since Phase 2) |
| RunState **serialization / resume** + `SmallRng` serde + score | Phase 8 persistence (§8.2 — switch to `rand_chacha` or seed+draw-count) |
| Non-param ThroneRoom curses needing bespoke consumers | as each curse's mechanic lands (the `extra_modifiers` path covers player-stat curses) |
| Act-3 secret level | later (feats of strength) |
| Character select | Phase 8 (menu flow) |

---

## 8. Risks & mitigations

| Risk | Mitigation / expected outcome |
|---|---|
| The auto-start / encounter systems move the golden master | D1: auto-start is windowed-only; encounter systems gate on `CurrentEncounter` (absent in the campaign). Adding gated-off systems is neutral under the movement pin (held across Phases 3–6). Verify byte-identical at every ★. |
| `extra_modifiers` wiring (7F) perturbs normal casts | With an empty `RoomModifiers`, `resolve_params` already receives `&[]` today — the change must be a no-op there. Verify the campaign is byte-identical after 7F. |
| Graph generation nondeterminism (HashMap iteration, `thread_rng`) | `build_act_graph` consumes **only `RunRng`**; when iterating a `HashMap<NodeId,…>` for anything gameplay-visible, sort by `NodeId` first (the map-signature pattern in the sim). Reproducibility tripwire guards it. |
| Scope creep (this is the biggest phase) | Sequence 7A–7G; ThroneRoom/Merchant isolated in 7F (cut to Phase 7.5 if needed, D2). Each step is independently shippable + compat-gated. |
| Themes reference non-existent enemies → encounters spawn nothing | D4: point pools at existing enemies + ship the `warlord` boss. A spawn from an unloaded id must degrade gracefully (skip + warn), never panic. |
| Per-encounter map regen breaks presentation (stale meshes) | Presentation-only; never headless. Re-render on `TileMap` change; specify in 7C but it can trail the logic (does not gate the golden master). |
| Teardown leaks entities across encounters (old enemies/zones persist) | Teardown despawns exactly the encounter-scoped markers (`Enemy`/`Projectile`/`PersistentZone`/`PickUp`/VFX); the player entity persists. Assert clean teardown in `objective_completion_advances_the_node`. |

---

## 9. As-built notes (to be completed on delivery)

_(Filled in after implementation, mirroring phase5/phase6 §9: which steps (if any) moved the baseline
and why, the resolved §0 decisions, deviations, final test count, and the §8.1/§8.9 debt updates.)_
