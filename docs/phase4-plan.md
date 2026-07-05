# Phase 4 Implementation Plan — Hero / Stance System + Mage (Focused Vertical Slice)

_Written 2026-07-05 against `main` @ `be8e6ba` (Phases 0–3.1 + testing infra complete).
Companion to `docs/architecture-plan.md` (§3.2 hero, §7 phase plan, §8.2/§8.5 amendments) and
`docs/testing.md`. As-built notes in §9._

---

## 0. Decisions locked for this phase

Three decisions were resolved with the project owner before planning; they set Phase 4's scope
(architecture-plan §7's two bullets drastically understate it — see §8.2).

| # | Decision | Consequence |
|---|---|---|
| **D1** | **Second class = Mage.** (§8.2's recommendation — reuses Phase-3 projectiles/status/stances with the least new machinery; Druid's enhanced-attack state machine + summons stay Phase 9.) | Fire/Ice stances bind the existing Fireblast/Frostbolt demonstrators. No new behavior primitive is needed for the core kit. |
| **D2** | **Scope = focused vertical slice.** The mandatory core only; the heavier Mage subsystems are deferred with explicit revival triggers (§7). | The hero/stance layer ships end-to-end with a fully-playable second class, without dragging in frost charges / Frost Impale / dash / real shields / code-driven hooks. |
| **D3** | **Playtest access = debug hotkey (M).** The Death Knight stays the default spawned hero (protects the golden-master baseline); a debug key re-identifies the player as the Mage. | The Mage is felt on the Windows build without a character-select screen (deferred), and the campaign bot never presses M so the baseline is untouched. |

---

## 1. Scope

### In scope (the mandatory core)
1. **`DefLibrary<T>` refactor** — pay the §8.5 "Def-library triplication" debt (owed "at Phase 4
   start") *before* `HeroDef` becomes a fourth copy.
2. **Hero module live** — `HeroDef` loader + `HeroLibrary`, `HeroPlugin`, stance/input-slot
   resolution, Q stance-swap.
3. **Death Knight formalized** as the default `HeroDef` (behavior-neutral: same level-1 grant,
   same LMB → `death_strike`).
4. **Mage** — Fire/Ice stances binding the existing `fireblast`/`frostbolt` basics; Q swaps
   Fire↔Ice.
5. **Stance-swap effects via the existing status system** — Ice Barrier = damage-reduction status;
   Boots of Fire = move-speed status.
6. **Debug hotkey (M)** to switch the live player to Mage for Windows playtesting.
7. **Presentation pass** (§8.5) — projectile sprites, status tints. (Nova flash deferred — §7.)
8. **Full test suite + docs.**

### Out of scope (explicitly deferred — see §7)
Frost-charge class resource + resource UI bar; Frost Impale + `channel_while_moving`; dash /
movement ability; real absorb/shield system (true Ice Barrier); code-driven status/ability hooks
+ the `execute_ready_abilities` resolve/apply split; `Override(0)` cooldown semantics; per-hero
base-stat application; character-select UI; full Mage progression content (Blaze, Flamewrath,
Frostbite, Frost charge, Flamestrike, talents).

---

## 2. Architecture

### 2.1 `DefLibrary<T>` — generic def registry (pay §8.5 first, behavior-neutral)
Before Phase 4, `AbilityLibrary`/`TalentLibrary`/`StatusLibrary` were three near-identical triples:
a `Resource { defs: HashMap<Id, Handle<T>> }` + `get()`, a byte-identical `AssetLoader` (only the
`Asset` type + extension differ), and a `load_x_defs` Startup system iterating a `const &[(id,
path)]`. One generic implementation (`src/core/def_library.rs`) replaces all three:

```rust
pub trait DefAsset: Asset + for<'de> Deserialize<'de> {
    const EXTENSIONS: &'static [&'static str];                    // e.g. &["ability.ron"]
    const MANIFEST: &'static [(&'static str, &'static str)];      // (id, asset_path) list
}
#[derive(Resource)] pub struct DefLibrary<T: Asset> { pub defs: HashMap<String, Handle<T>> }
pub struct RonDefLoader<T: DefAsset>(PhantomData<fn() -> T>);     // one loader, ron::de::from_bytes
// App ext: register_def_library::<T>() = init_asset + register_asset_loader + init_resource
//          + Startup populate_def_library::<T>, in one call.
```

Each concrete def keeps its public name via a **type alias** (`pub type AbilityLibrary =
DefLibrary<AbilityDef>;`, …), so no downstream `Res<AbilityLibrary>` / `library.get(id)` /
`library.defs` call site changes, and the sim harness accessors keep working unchanged. The
per-type `impl DefAsset` (extension + manifest) lives in each `assets.rs`; each plugin's four-line
boilerplate collapses to `app.register_def_library::<XDef>();`. **Pure refactor — byte-identical
behavior**, gated on an unchanged golden baseline.

### 2.2 Hero module + indirection
- `HeroDef` gains `serde::Deserialize` (the scaffold omitted it) + `impl DefAsset` with the
  `hero.ron` compound extension; `assets/heroes/blood_death_knight.ron` → `.hero.ron`. One field
  added to `StanceSlotMapping`: `#[serde(default)] swap_effect: Option<StatusEffectId>` (the status
  applied when *entering* that stance — keeps the swap effect data-driven).
- `HeroPlugin` (into `GameLogicPlugin`) registers `HeroDef` via `register_def_library`, and runs
  `resolve_input_to_ability` + `handle_stance_swap` `.before(CombatSet::Damage)`, InRun-gated.
- **`resolve_input_to_ability`** reads `(HeroIdentity, ActiveStance)`, looks up the active stance's
  `StanceSlotMapping` (via a pure, unit-tested `resolve_slot`), maps **LMB→basic**, **RMB→special**,
  and emits `TriggerAbilityEvent`. Replaces the Phase-1 stub `player/systems/ability_input.rs`
  (deleted).
- **`handle_stance_swap`** on Q: if `has_stance`, toggle `ActiveStance` between `stance_a`/`stance_b`
  and emit `ApplyStatusEvent` for the entered stance's `swap_effect` (reuses the status apply
  pipeline — no new system). No-op for non-stance heroes.
- **`spawn_player`** now also inserts `HeroIdentity(DEFAULT_HERO_ID = "blood_death_knight")` +
  `ActiveStance::default()` ("default").

### 2.3 Deferred, HeroDef-sourced level-1 grant
`grant_level_1_abilities` re-sources its list from `HeroDef.level_1_abilities` instead of a
hardcoded array. Because `HeroDef` loads asynchronously, the grant is **deferred**: it runs for any
player `Without<Level1Granted>`, and once its `HeroDef` handle resolves it emits the unlocks and
inserts the `Level1Granted` marker (fires once per player). `sim::settle_assets` waits for both the
`HeroLibrary` handles *and* the grant to complete, so `new_arena(...)` returns with starting
abilities in place. For the Death Knight this yields the identical `["death_strike","dnd",
"companion"]` set → baseline-neutral. (Idle settle frames don't advance `RunRng` — spawners paused,
no input — so deferring the grant a few frames doesn't move the campaign.)

### 2.4 Stance-swap effects via the status system
Rather than build the real absorb shield (deferred), the two swap effects are ordinary status
defs the existing system already supports:
- **Boots of Fire** (entering Fire) — `move_speed_mult 1.3`, 3s. A move-speed buff, folded into
  `MoveSpeedModifier` by `resolve_actor_status` exactly like frostbite's slow.
- **Ice Barrier** (entering Ice) — `damage_taken_mult 0.6`, 3s. A −40% damage-reduction buff,
  standing in for Mechanics.md's "absorb the next attack/projectile" until the real absorb lands.

### 2.5 Debug hotkey (playtest access)
`hero/systems/debug.rs::debug_swap_to_mage` (mirrors `debug_force_level_up`, `#[cfg(debug_
assertions)]`): on **M**, sets `HeroIdentity("mage")` + `ActiveStance("fire")` and removes
`Level1Granted` so the deferred grant re-runs and hands over the Mage's kit. Compiled out of
release; the campaign bot never presses M.

### 2.6 Presentation pass (pure presentation, baseline-neutral)
Registered only in `PresentationPlugin` (never runs headless → cannot move the baseline):
- **`attach_projectile_visuals`** (`Added<ProjectileMotion>`) — attaches a small circle mesh
  tinted by the projectile's elemental damage tag; `sync_transform` then follows its WorldPosition.
- **`tint_status_effects`** — recolors each enemy's own material toward its active status's color
  (frostbite blue, blaze orange, bleed red, root/stun yellow), reverting on expiry.

The **Blood Boil nova flash** is deferred: the melee-cone flash is spawned via the *logic* path
(`execute.rs`), so a nova flash the same way would spawn new entities during the DK campaign and
move the baseline. A baseline-neutral nova flash needs a presentation-only cast-VFX event bus — a
small future item (§7), not worth a baseline regen this slice.

---

## 3. File-level change map

| Area | File(s) | Change |
|---|---|---|
| Def library | `core/def_library.rs` (new), `core/mod.rs` | generic `DefLibrary<T>` + `DefAsset` + `RonDefLoader<T>` + `register_def_library` |
| Def library | `ability/assets.rs`, `talent/assets.rs`, `status/assets.rs` | replace `XLibrary` struct + `XDefLoader` with a type alias + `impl DefAsset` (extension + manifest) |
| Def library | `ability/plugin.rs`, `talent/plugin.rs`, `status/plugin.rs` | `load_x_defs` + wiring → `app.register_def_library::<XDef>()` |
| Hero | `hero/assets.rs` | `HeroDef` Deserialize + `impl DefAsset`; `HeroLibrary` alias; `swap_effect` field; parse tests |
| Hero | `hero/plugin.rs`, `hero/mod.rs` | implement `HeroPlugin`; export it |
| Hero | `hero/systems/input_slot.rs`, `stance.rs` | implement `resolve_input_to_ability` (+ pure `resolve_slot`) and `handle_stance_swap` |
| Hero | `hero/components.rs` | `DEFAULT_HERO_ID` const |
| Hero | `hero/systems/debug.rs` (new), `hero/systems/mod.rs` | `debug_swap_to_mage` (M) |
| Wiring | `lib.rs`, `game/plugin.rs` | declare `hero`; add `HeroPlugin` to `GameLogicPlugin` |
| Player | `player/systems/spawn_player.rs` | insert `HeroIdentity` + `ActiveStance` |
| Player | `player/plugin.rs`, `player/systems/{mod,ability_input}.rs` | drop the Phase-1 LMB stub (deleted) |
| Ability | `ability/plugin.rs`, `ability/components.rs` | deferred HeroDef-sourced grant + `Level1Granted` marker |
| Presentation | `projectile/systems/visuals.rs` (new), `status/systems/visuals.rs` (new), `game/presentation.rs`, the two `systems/mod.rs` | projectile sprites + status tints |
| Sim | `src/sim/mod.rs` | `assets_loaded` awaits `HeroLibrary` + grant; `set_hero`/`hero_id`/`active_stance`/`tap_mouse` helpers |
| Content | `assets/heroes/mage.hero.ron` (new), `blood_death_knight.hero.ron` (renamed) | Mage + DK defs |
| Content | `assets/status_effects/{boots_of_fire,ice_barrier}.status.ron` (new) | swap-effect statuses |
| Tests | `tests/hero_stance.rs` (new) | 6 golden scenarios |

---

## 4. Content

### 4.1 Heroes (`*.hero.ron`)
| id | has_stance | stances | level_1 | resource | notes |
|---|---|---|---|---|---|
| blood_death_knight | false | default | death_strike, dnd, companion | HealthBased | LMB→death_strike, RMB→dnd; Q unbound |
| mage | true | fire / ice | fireblast, frostbolt | None | fire: LMB→fireblast, swap→boots_of_fire; ice: LMB→frostbolt, swap→ice_barrier |

Mage band/passive pools are empty this slice; Special (Flamestrike/Frost Impale) and Movement
(dash) slots are unbound. `base_stats` are data-only (not yet applied to runtime Health/speed).

### 4.2 Stance-swap statuses (`*.status.ron`)
| id | stacking | dur | move× | dmg-taken× | notes |
|---|---|---|---|---|---|
| boots_of_fire | RefreshOnReapply | 3.0 | 1.3 | 1.0 | Ice→Fire buff |
| ice_barrier | RefreshOnReapply | 3.0 | 1.0 | 0.6 | Fire→Ice buff; stand-in for the deferred absorb |

_(Numbers are proposed defaults, tunable; recorded in Mechanics.md.)_

---

## 5. Implementation sequence (each step is independently `/compat-check`-able)

**Step 1 — `DefLibrary<T>` refactor (behavior-neutral).** ★ Gate: green, **baseline unchanged**.
**Step 2 — Hero module live + DK formalized.** ★ Gate: **baseline unchanged** (highest-risk gate —
the deferred grant + input indirection are the only places behavior could shift; the campaign
bypasses the input layer, so it doesn't).
**Step 3 — Mage content + swap effects + debug hotkey + `tests/hero_stance.rs`.** ★ Gate: green.
**Step 4 — Presentation pass** (Windows-verified visuals; headless unaffected). ★ Gate: green.
**Step 5 — Docs** (this file's as-built, CHANGELOG, architecture-plan §8.5/§8.6, Mechanics,
testing.md, CLAUDE.md). Final `/compat-check`.

---

## 6. Validation & testing suite

### 6.1 Unit tests (`src/**` `#[cfg(test)]`)
- `core/def_library.rs` — `DefLibrary::get` hit/miss.
- `hero/assets.rs` — parse `blood_death_knight.hero.ron` + `mage.hero.ron` (id, has_stance,
  stances, level_1, stance_slots, swap_effect).
- `hero/systems/input_slot.rs` — pure `resolve_slot`: DK default slots; Mage fire→fireblast /
  ice→frostbolt; unknown stance / unbound slot → None.

### 6.2 Golden scenarios (`tests/hero_stance.rs`)
1. `default_death_knight_lmb_casts_death_strike` — regression guard: the indirection preserves DK.
2. `second_class_basic_attack_fires_through_input_slots` — Mage LMB fires the active stance's basic.
3. `stance_swap_remaps_lmb` — same LMB → blaze (Fire) vs frostbite (Ice) after Q.
4. `stance_swap_applies_entering_stance_effect` — Q applies Ice Barrier / Boots of Fire; Ice
   Barrier mitigates 50→30 (×0.6).
5. `non_stance_hero_q_is_a_noop` — DK Q changes nothing.
6. `debug_hotkey_switches_player_to_mage` — M re-identifies + grants the Mage kit.

### 6.3 Golden master
Unchanged (still the Death Knight bot). **Baseline does not move** — the refactor and indirection
are engineered baseline-neutral, and the campaign bypasses the input layer. No regeneration.

### 6.4 Compat gate
`/compat-check` at every ★. Any baseline movement = regression to classify (there was none).

---

## 7. Deferred — with the trigger that revives each

| Deferred | Revived by |
|---|---|
| Frost-charge class resource + resource UI bar | Mage capstone content ("Core + capstone" scope / Phase 4.x) |
| Frost Impale + `channel_while_moving` primitive | same |
| Dash / movement ability (`InputSlot::Movement`) | first class that needs a dash |
| Real absorb/shield system (true Ice Barrier) | first true absorb (bone shield / Paladin overheal / Mage) |
| Code-driven status/ability hooks + `execute_ready_abilities` split | first code-driven hook (none land this slice) |
| `Override(0)` cooldown semantics | first cooldown-to-zero talent |
| Per-hero base-stat application (Health/move-speed from `HeroDef.base_stats`) | when class HP/speed differentiation matters (feel/balance) |
| Blood Boil nova flash VFX | a presentation-only cast-VFX event bus (logic-path spawn would move the baseline) |
| Character-select UI (`GameState::CharacterSelect`) | later phase |
| Full Mage progression content (Blaze, Flamewrath, Frostbite, Frost charge, Flamestrike, talents) | Phase 9 content pass |

---

## 8. Risks & mitigations

| Risk | Mitigation / outcome |
|---|---|
| Deferred grant / input indirection perturbs the DK baseline (Step 2) | DK's `level_1_abilities` are byte-identical; campaign uses `trigger_ability` directly (never LMB). **Result: baseline unchanged.** |
| `HeroDef` deserialization (scaffold lacked `Deserialize`; `ResourceModel::None` vs `Option::None`) | RON resolves by target type; the parse unit tests cover both heroes. |
| Loader extension collision on plain `.ron` | `hero.ron` compound extension + file rename. |
| Grant races `settle_assets` (harness checks abilities right after `new_arena`) | `assets_loaded` waits for the grant too. |
| RMB→`dnd` for DK (unregistered `dropped_zone` behavior) | graceful no-op (existing "unregistered-behavior skip" test); no panic, no baseline effect. |
| Presentation systems move the baseline | pure presentation (never in `GameLogicPlugin`) → provably headless-neutral. |

---

## 9. As-built notes (completed 2026-07-05)

Phase 4 landed as planned. Highlights and deviations:

- **`DefLibrary<T>` via type aliases** — the three `XLibrary` structs became
  `pub type XLibrary = DefLibrary<XDef>;`, so every `Res<AbilityLibrary>` / `library.get(id)` /
  `library.defs` call site and the three sim accessors compiled unchanged. Manifests moved into
  each `assets.rs` as `impl DefAsset`. Step 1 gate: baseline byte-identical.
- **The Death Knight was already the de-facto default hero** — the old grant stub granted exactly
  `blood_death_knight.hero.ron`'s `level_1_abilities`, so formalizing it was behavior-neutral.
- **Deferred grant + `settle_assets` wait** — the only real baseline risk (Step 2). Confirmed
  neutral: idle settle frames advance no `RunRng`, and the campaign bot bypasses the input layer.
  Baseline did NOT move across Steps 1–4.
- **Stance-swap effects modeled as statuses** — Boots of Fire (move-speed) and Ice Barrier
  (damage-reduction) reuse the Phase-3 status system entirely; no new machinery. The real
  next-hit absorb is deferred (§7).
- **Presentation pass split from the nova flash** — projectile sprites + status tints landed as
  pure presentation (baseline-neutral). The nova flash was deferred because the existing cone-flash
  path is logic-side and would move the baseline; it needs a presentation-only cast-VFX bus.
- **Per-hero base-stat application deferred** — `HeroDef.base_stats` stays data-only this slice
  (spawn still uses the shared constants); the Mage plays with the shared HP/speed. Noted in §7.
- **Tests: 84 passing** (was 73). Build warning-free. Golden baseline unchanged (no regeneration).
- **Debt resolved (see architecture-plan §8.5/§8.6):** Def-library triplication (done); presentation
  pass (projectiles + tints done, nova flash re-filed). Re-filed as deferred with triggers: the
  `execute_ready_abilities` split (no hook landed this slice), `Override(0)` cooldown semantics,
  dash, shields, frost charges, code-driven hooks. Wall-collision remains owner-accepted.
