# Phase 8 Plan — Persistence + Meta

_Written 2026-07-05, after Phase 7.5. Plan portion first; **as-built notes** get appended after
implementation (the template every phase doc follows). Source of truth for scope, decisions, and the
work breakdown. Read `docs/architecture-plan.md` §3.10 (dual-scope persistence), §3.11 (seeded RNG),
§8.2 (Phase-8 corrections), §8.10 (Phase 7.5 carve-outs) first._

---

## 0. Scope

Phase 8 closes the persistence + meta surface deferred through Phases 7 and 7.5:

- **RunState serialization** — save on node transition, load on "Resume Run" (architecture §7 Phase 8.1).
- **MetaState** — hero unlocks + scoreboard (+ the score formula, §8.1(10)); light up the greyed
  main-menu buttons (Resume / Scoreboard) and hero greying on character select (§7 Phase 8.2).
- **Resume Run** from the main menu; **Log-In** local profile; move player/map spawn from `Startup`
  to `OnEnter(InRun)` (the `game/state.rs` TODO) (§7 Phase 8.3, §8.10 carve-outs).

### Owner decisions (resolved 2026-07-05, before implementation)

| # | Decision | Choice | Consequence |
|---|---|---|---|
| **D1** | RunRng save/resume fidelity (§8.2 open item) | **Exact resume via `rand_chacha`** — switch `RunRng(SmallRng)` → `RunRng(ChaCha8Rng)` (serde), serialize the full RNG state | **Golden master regenerates** (a declared change — the first baseline move since Phase 3). Resume is bit-identical, as architecture §3.11 intended. New direct dep `rand_chacha` (already transitive). |
| **D2** | Scoreboard score formula (§8.1(10)) | **Progress + speed** — act/node depth + level + victory bonus, plus a time bonus (faster = higher) | Adds a deterministic **run timer** (`elapsed_secs`) to `RunState` (serialized, resume-safe). |
| **D3** | Hero unlock rule | **All unlocked (mechanism only)** — every defined hero starts unlocked; the greying path + `unlocked_heroes` exist and are tested, but nothing is locked yet | Concrete unlock **triggers are Phase 9** (with the real hero roster). Matches the current "all unlocked for now" note. |
| **D4** | Optional scope | **IN:** Log-In profile screen + orphaned-`AbilityInstance` fix. **OUT (→ Phase 9):** per-hero `base_stats` application | Log-In gets its own `GameState::Login`. The orphan fix is byte-identical. `base_stats` stays deferred (applying it would be a *second* golden regen — DK 100→200 HP — and is a balance concern). |

Standing constraints (architecture §6): **local-only** persistence (Q3); **thin** MetaState — no
currency, no permanent power (Q2). Power fully resets each run.

### Definition of done

1. `RunState` + `MetaState` serialize/deserialize (RON, serde) and round-trip in unit tests.
2. A run **saves** on every node transition and on run end; **Resume Run** reconstructs a live run
   byte-for-byte (the D1 payoff — the resumed roster/offers/drops match a never-interrupted timeline).
3. Scoreboard shows finished runs ranked by a **progress+speed** score; Login → Main menu → Resume /
   Scoreboard buttons are live; character select greys locked heroes (none locked yet) and refuses a
   locked pick.
4. Player/map spawn moved to `OnEnter(InRun)` with an idempotent guard; orphan `AbilityInstance` leak
   fixed.
5. **The golden master is regenerated exactly once (D1 RNG switch), declared in CHANGELOG**, and every
   other step is byte-identical relative to that new baseline. Build stays warning-free; `/compat-check`
   is green (RNG divergence classified as the declared change, nothing else).
6. Docs updated: this plan's as-built notes, architecture §8.11 + §8.5/§8.2/§7 edits, CHANGELOG "Phase
   8", `docs/testing.md`, `Mechanics.md`, `CLAUDE.md`, and `lib.rs` declares `pub mod meta;`.

---

## 1. Data-structure changes

### 1.1 `RunRng` → ChaCha8 (`run/rng.rs`)

`SmallRng` appears **only** in `run/rng.rs` (the newtype) — every consumer goes through `rng.rng()` and
uses the `Rng` trait, so the switch is a two-line type change with zero consumer churn.

```rust
use rand_chacha::ChaCha8Rng;              // 0.3.1, feature "serde1"

#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct RunRng(pub ChaCha8Rng);

impl RunRng {
    pub fn from_seed(seed: u64) -> Self { Self(ChaCha8Rng::seed_from_u64(seed)) }
    pub fn rng(&mut self) -> &mut ChaCha8Rng { &mut self.0 }
}
```

`Cargo.toml`: add `rand_chacha = { version = "0.3.1", features = ["serde1"] }` (pairs with `rand 0.8.6`,
already resolved in `Cargo.lock`).

**Bonus determinism win (document it):** `SmallRng` is explicitly *not* portable/stable across rand
versions or platforms; `ChaCha8Rng` **is** value-stable. This strengthens the golden-master portability
note in `docs/testing.md` rather than weakening it.

### 1.2 `RunState` (`run/state.rs`) — serde + fidelity

- Add `#[derive(Serialize, Deserialize)]` to `RunState`, `ObjectiveProgress` **is not serialized** (it's
  live-encounter state, rebuilt from the graph on load) — serialize `RunState` only, keep
  `CurrentEncounter` transient.
- Add **`elapsed_secs: f32`** (D2) — a deterministic run clock, accumulated from `Time::delta` while
  `InRun` + a run exists (never in the runless campaign).
- **Fidelity fix (blocker for resume):** today `handle_encounter_complete` syncs only
  `player_health`/`player_level` — `unlocked_abilities` and `acquired_talents` are *never* written, so
  they'd serialize empty. Add a `sync_run_state` step that reads the live player's `AbilityInstance`
  children → `unlocked_abilities` and `AcquiredTalents` → `acquired_talents` (and `elapsed_secs`,
  `level_flow`) at every save point.

Nested types needing `#[derive(Serialize, Deserialize)]`: `ActGraph`, `EncounterNode`, `EncounterType`,
`ObjectiveType` (`world/graph.rs`); `LevelUpFlowState`, `LevelUpPhase` (`progression/state.rs`);
`TalentOffer` (`talent/offer.rs`); `StatModifier`, `ModOp`, rarity/uniqueness enums as referenced
(`talent/assets.rs`). All are plain data — derives only, no manual impls. `HashMap<NodeId,…>` and
`Vec<…>` serialize natively via RON.

### 1.3 `MetaState` (`meta/state.rs`) — serde + nested run

- `#[derive(Serialize, Deserialize)]` on `MetaState` and `RunRecord`.
- Change `in_progress_run: Option<Vec<u8>>` → **`Option<SavedRun>`** (nested, human-inspectable RON, not
  opaque bytes).
- `unlocked_heroes` initialized to **all** `HeroDef::MANIFEST` ids on first launch (D3).

```rust
/// The complete on-disk snapshot of an in-progress run: the run record + its RNG stream position,
/// bundled so resume is bit-exact (D1). Split back into the RunState + RunRng resources on load.
#[derive(Serialize, Deserialize, Clone)]
pub struct SavedRun { pub run: RunState, pub rng: RunRng }
```

### 1.4 New `GameState::Login` (`game/state.rs`)

Add a `Login` variant (D4). Windowed boot becomes **Login → Menu → CharacterSelect → run**. Headless
sim never enters Login (same gating as Menu today) ⇒ campaign untouched.

---

## 2. Persistence architecture (the load-bearing decision)

Mirror the project's logic/presentation split: **pure (serde) layer** unit-tested with no disk; **thin
disk layer** the headless path never touches.

```
meta/persistence.rs
  serialize_meta(&MetaState)   -> Result<String, _>     // RON, pure, no I/O
  deserialize_meta(&str)       -> Result<MetaState, _>  // pure; caller maps Err → default
  save_path()                  -> PathBuf               // env override → platform dir → ./saves
  save_meta_to_disk(&MetaState)                         // thin wrapper over serialize_ + fs::write
  load_meta_from_disk()        -> MetaState             // read → deserialize → default on missing/corrupt
```

- **Save path resolution (no new dep):** `RUSTGAME_SAVE_DIR` env override (tests/CI point it at a temp
  dir) → platform default (`%APPDATA%` on Windows, `$XDG_DATA_HOME` / `$HOME/.local/share` on Unix) →
  final fallback `./saves`. `directories` crate is an easy later swap if the std resolver proves thin.
- **Headless safety:** the *disk* systems are added by **`GamePlugin` (windowed)**, not
  `GameLogicPlugin`. The sim drives save/resume through the **pure** layer + in-memory
  `MetaState.in_progress_run` — no filesystem, fully deterministic. `MetaState` itself (the resource) is
  inserted by `MetaPlugin` in `GameLogicPlugin` so the logic is sim-able; only the *I/O* is windowed.
- **Corrupt/missing file** → `MetaState::default()` (first-run behavior). Never panics.

---

## 3. Save cadence & Resume hydration

### 3.1 When we save

At every **node boundary** (the natural roguelite save point), driven off the existing lifecycle:

- On `EncounterCompleteEvent` / entering `MapSelect` / act advance → `sync_run_state` then snapshot
  `SavedRun { run, rng }` into `MetaState.in_progress_run`; windowed → `save_meta_to_disk`.
- On **run end** (death in `player_death`, or Act-3 victory in `handle_encounter_complete`) → append a
  `RunRecord` (with the computed score), **clear** `in_progress_run`, save.

Mid-encounter progress is intentionally **not** saved: quitting mid-room resumes at that room's start
(the saved `current_node`), reloaded fresh. Standard for the genre and keeps the snapshot small.

`RunRecord.timestamp_unix` uses `SystemTime` in the windowed run-end path only (never asserted in
headless tests; the campaign is runless).

### 3.2 Resume = deterministic re-hydration

`resume_run(world, saved: SavedRun)` — the mirror of `reset_and_start_run`, reusing its primitives:

1. `teardown_run(world)` (existing) — clean slate.
2. Insert `RunState` (from `saved.run`) + `RunRng` (from `saved.rng` — **exact stream position**) +
   `RoomModifiers::default()`.
3. `respawn_player` (existing) → set `HeroIdentity`, `ActiveStance` (saved hero's `stance_a`/default),
   `Health.current = run.player_health`, `Experience.level = run.player_level`.
4. **Re-grant abilities:** emit one `UnlockAbilityEvent` per `run.unlocked_abilities` — reuses the
   idempotent `spawn_unlocked_ability` path (plus `grant_level_1_abilities` for L1). No new spawn code.
5. **Re-install talents:** replay `run.acquired_talents` through the existing `install_acquired_talent`
   path (rebuilds `AcquiredTalents` + modifier cache + `ActiveHooks`).
6. `LevelUpFlowState` ← `run.level_flow` (already inline).
7. Rebuild `CurrentEncounter::for_node(current_node, depth)` from the saved `act_graph`; set `InRun`;
   `load_encounter` loads the saved node fresh next frame — and because the RNG stream is restored
   exactly, the roster it rolls is **identical to the uninterrupted run** (the D1 payoff, and the
   headline scenario test).

Wire the main-menu **Resume Run** input: enabled iff `MetaState.in_progress_run.is_some()`; emits a
`ResumeRunRequest`, consumed by an exclusive `apply_resume_request` (mirrors `apply_start_run_request`).

---

## 4. MetaState surfaces (screens & gating)

- **`unlocked_heroes`** seeded to all `HeroDef::MANIFEST` ids at first launch; a
  `hero_is_unlocked(&MetaState, id) -> bool` predicate (unit-tested with a deliberately-locked hero).
- **Character select** (`ui/screens/character_select.rs`): render locked heroes greyed via the
  predicate (none locked today). **`handle_character_select_input`** refuses a locked pick (gate the
  `StartRunRequest`). Logic-side + testable.
- **Scoreboard** (`ui/screens/scoreboard.rs`, new `GameState::Scoreboard`): read-only list of
  `run_history` sorted by score desc; Esc → Menu. Presentation-only; the score math is a pure function.
- **Hero-unlock hook:** a no-op `unlock_heroes_on_progress` stub called at run end (the Phase-9 seam —
  where win/act triggers will live). Documented as inert this phase (D3).
- **Menu wiring** (`run/systems/menu.rs`): Resume (if a save exists) → `ResumeRunRequest`; Scoreboard →
  `GameState::Scoreboard`; both were greyed placeholders in Phase 7.5.

### Score formula (D2, pure fn, tunable consts)

```
progress = act*1000 + node_column*50 + level*100 + (if victory {5000} else {0})
speed    = max(0.0, TIME_PAR_SECS - elapsed_secs) * SPEED_WEIGHT   // faster clear ⇒ higher
score    = (progress as f32 + speed).round() as u32
```

Constants live in `meta/` (or `constants.rs`); trivially retuned without touching call sites.

---

## 5. Cleanups (D4 + the Startup→OnEnter move)

- **Player/map spawn → `OnEnter(InRun)`** (`game/state.rs` TODO): move `spawn_player`, `generate_map`,
  `init_level_flow` (keeping the `init_level_flow.after(generate_map)` order) from `Startup` to
  `OnEnter(GameState::InRun)`, **guarded "spawn only if absent."** Rationale + safety:
  - Headless sim boots into `InRun` (default) and `Sim::new` already pumps one frame, so the initial
    `StateTransition` fires `OnEnter(InRun)` — player/map exist before any test queries them.
  - The idempotent guard makes overlay round-trips (`Paused`→`InRun`, `MapSelect`→`InRun`) and the
    windowed reset/resume paths (which spawn their own player) **not** double-spawn.
  - Windowed boot now sits in Login/Menu with **no** live simulation underneath (the TODO's real goal).
  - Because 8A already regenerates the golden master, any *deterministic* re-timing here is absorbed by
    that one regen — but we verify it introduces **no nondeterminism** (reproducibility test stays green)
    and no *further* trace drift beyond the RNG switch (do 8A's regen first, then land this and re-check).
- **Orphaned `AbilityInstance` fix** (§8.5 register row): in `enemy_death`, despawn the dying enemy's
  owned `AbilityInstance` entities (filter by `owner`); add `AbilityInstance` to
  `despawn_encounter_entities`. Not a golden-trace field ⇒ **byte-identical**. Resolves the last-but-one
  §8.5 row (base_stats remains, deferred to Phase 9).

---

## 6. Work breakdown (ordered; each step ends green)

| Step | Deliverable | Golden master |
|---|---|---|
| **8A** | RNG swap to `rand_chacha` (`Cargo.toml` + `run/rng.rs`). Regenerate + commit the baseline. | **REGENERATED (declared)** |
| **8B** | serde derives across the RunState object graph (§1.2 nested types). Unit: `RunState` round-trips. | byte-identical (vs 8A) |
| **8C** | Persistence layer (§2): pure serialize/deserialize, `SavedRun`, save path, disk wrapper, `MetaPlugin` into `GameLogicPlugin`, real `load_meta`, `lib.rs` `pub mod meta;`. Unit: round-trip, corrupt→default, path resolution. | byte-identical |
| **8D** | Fidelity + cadence + scoring (§1.2, §3.1, §4 formula): `sync_run_state`, `elapsed_secs` timer, save on transitions + run end, `RunRecord` + score. Scenario: state synced; score computed. | byte-identical |
| **8E** | Resume (§3.2): `resume_run` hydration, `ResumeRunRequest`, menu wiring. Scenario: save→resume is bit-exact. | byte-identical |
| **8F** | MetaState surfaces (§4): `unlocked_heroes` (all) + predicate, character-select greying + locked-pick gate, scoreboard screen/state, unlock-hook stub. Scenario: locked pick refused; run-end appends a scored record. | byte-identical |
| **8G** | Login (§1.4): `GameState::Login`, `login.rs` screen, boot Login→Menu, input handler. Scenario: boot reaches Login then Menu. | byte-identical |
| **8H** | Cleanups (§5): Startup→`OnEnter(InRun)` idempotent move; orphan `AbilityInstance` fix; register/doc updates. Scenario: no further drift; reproducibility green. | byte-identical |

Dependency order: 8A→8B→8C→8D→8E; 8F/8G/8H layer on top. Do **8A in isolation and regenerate first** so
the single declared baseline move is cleanly attributable; every later step must re-verify byte-identity
against that new baseline.

---

## 7. Testing suite (definition of done per §8.3)

### Unit tests (`src/**` `#[cfg(test)]`)
- `RunState` / `MetaState` / `SavedRun` RON round-trip (serialize→deserialize→equal).
- **RNG stream restore:** seed a `RunRng`, draw N, snapshot, draw M more, restore the snapshot, assert
  the next M draws match — proves bit-exact resume (the D1 contract) at the type level.
- `ChaCha8Rng` determinism: same seed ⇒ same sequence (value-stability pin).
- Save-path resolution (env override honored; fallbacks) and corrupt/missing → `default()`.
- Score formula (pure fn) across act/node/level/victory/time cases; `hero_is_unlocked` predicate.

### Golden scenarios (`tests/*.rs`, through the real input/systems via `Sim`)
- **`tests/persistence.rs` (new):**
  - "run state syncs abilities + talents + timer into `RunState` at a node transition."
  - "save → resume reconstructs a live run" — health/level/abilities/talents/node/act all match.
  - **"resume continues the RNG stream exactly"** — the encounter roster after a save/resume equals the
    roster of an uninterrupted run (headline test of D1; the whole reason for `rand_chacha`).
  - "resume with no/absent save falls back cleanly (stays in menu, no panic)."
- **`tests/meta.rs` (new) or extend `game_flow.rs`:**
  - "a locked hero pick is refused" (lock a hero in `MetaState`, assert no `StartRunRequest`).
  - "run end appends a `RunRecord` with a computed score" (defeat + victory paths).
  - "scoreboard reads `run_history` sorted by score."
- **`tests/game_flow.rs` (extend):** "boot reaches Login then Menu" (windowed state flow, driven
  headless like the existing character-select test); "Resume from the main menu enters `InRun` with the
  saved run."

### Golden master & compat
- Regenerate `tests/golden/campaign_baseline.ron` **once** at 8A: `UPDATE_GOLDEN=1 cargo test --test
  golden_campaign`, committed with the CHANGELOG "Phase 8" entry that explains the RNG switch.
- After every subsequent step, `cargo test` must pass with **no further** baseline change.
- `campaign_is_reproducible_within_a_build` must stay green throughout (ChaCha is deterministic; any
  failure = leaked nondeterminism, fix the source, don't regen).
- Run **`/compat-check`** at the end: it must classify the campaign divergence as the single declared
  RNG change and find nothing else.

---

## 8. Documentation updates (land with the code, same commit)

- **This file** — append the "As-built" section (deviations, deferrals, per-step notes) after
  implementation, like every prior phase doc.
- **`docs/architecture-plan.md`** — new **§8.11 "Phase 8 delivered"**; flip §7 Phase 8 bullets to done;
  resolve §8.2's Phase-8 note (record the `rand_chacha` choice); in §8.5 mark the orphan-`AbilityInstance`
  row **RESOLVED**, leave `base_stats` as the last open row (→ Phase 9); update the §8.10 carve-out list.
- **`CHANGELOG.md`** — "Phase 8" section (the behavior contract): the **RNG algorithm switch + baseline
  regeneration** (declared), persistence/save/resume, scoreboard + score formula, Login, hero-unlock
  mechanism, the Startup→OnEnter move, the orphan fix.
- **`docs/testing.md`** — note the baseline regen + its cause; add the Phase-8 scenarios to "Adding
  scenarios"; upgrade the portability note (ChaCha value-stability).
- **`Mechanics.md`** — mark Log in / Resume run / Scoreboard (+ formula) / unlock-greying as implemented.
- **`CLAUDE.md`** (repo) — add `docs/phase8-plan.md` to the map; update the debt highlights; **memory
  `MEMORY.md`** — bump the phase pointer.
- **`src/lib.rs`** — declare `pub mod meta;` (the module joins the crate this phase).

---

## 9. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Golden regen contaminated by accidental drift (not just the RNG) | Land **8A alone**, regenerate, commit; every later step re-runs the ladder and must be byte-identical vs. the new baseline. A second regen is a red flag to investigate, not to accept. |
| `Startup`→`OnEnter(InRun)` breaks "player exists at t=0" for tests | `Sim::new` already pumps one frame (initial `StateTransition` fires `OnEnter(InRun)`); idempotent guard covers overlay/reset/resume round-trips; full ladder validates. If any fixture regresses, pump the transition in `Sim::new` explicitly. |
| Filesystem I/O leaking into headless tests (nondeterminism, litter) | Disk systems live in `GamePlugin` (windowed) only; the sim exercises the **pure** layer + in-memory `in_progress_run`; `RUSTGAME_SAVE_DIR` isolates any disk-touching unit test to a temp dir. |
| Resume hydration double-spawns or drifts | Reuse `teardown_run` + `respawn_player` + the idempotent `spawn_unlocked_ability`/`install_acquired_talent` paths; assert a clean entity census in the resume scenario (as `restart_after_death` already does). |
| `RunState` still serializing empty abilities/talents | `sync_run_state` (§1.2) is a prerequisite of 8D — resume scenario asserts the talents/abilities survive the round-trip, which fails loudly if the sync is missing. |
| `rand_chacha` version drift | Pin `0.3.1` (already in `Cargo.lock`, pairs with `rand 0.8.6`); baseline is `Cargo.lock`-pinned per `docs/testing.md`. |

---

## 10. Out of scope (explicit, → Phase 9 or later)

- Per-hero `base_stats` application (D4-OUT; a second golden regen + balance concern).
- Concrete hero-unlock triggers (D3; needs the real roster).
- Multi-profile Login / any networked/cloud save (§6 Q3: local, single profile, serde-swappable).
- Settings screen (nothing to configure), a separate Heroes gallery (character select covers it),
  mouse input, damage numbers / minimap / tooltips (later UX/art).

---

## 11. As-built notes (completed 2026-07-06)

Phase 8 landed as planned across 8A–8H, at **full scope**. **The golden master moved exactly once**
(8A, the declared RNG-algorithm regeneration) — every step from 8B onward re-verified byte-identical
against the new baseline, and `campaign_is_reproducible_within_a_build` stayed green throughout (no
leaked nondeterminism from any Phase-8 system). See the CHANGELOG "Phase 8" section and
architecture-plan §8.11.

- **§0 decisions (resolved as planned).** **D1** exact resume via `rand_chacha::ChaCha8Rng`,
  hand-serialized. **D2** progress+speed scoreboard formula, with the run timer added to
  `RunState`. **D3** every `HeroDef::MANIFEST` hero ships unlocked; the lock/unlock mechanism and
  its UI/refusal path are fully wired and unit/scenario-tested against a *deliberately* locked hero
  (`Sim::lock_hero`), since no hero is locked by default. **D4** Log-In + the orphan-`AbilityInstance`
  fix landed in-scope; per-hero `base_stats` stayed out.

- **Deviation: hand-rolled `RunRng` serde, not `rand_chacha`'s `serde1` feature.** The plan's §1.1
  sketch suggested `rand_chacha = { features = ["serde1"] }`. That feature's `Serialize` impl encodes
  the word-position as a `u128`, and `ron` 0.8 cannot serialize `u128`/`i128` at all ("u128 is not
  supported") — discovered while implementing 8A. `run/rng.rs` instead implements `Serialize`/
  `Deserialize` by hand against `ChaCha8Rng`'s public `get_seed`/`get_stream`/`get_word_pos`/
  `set_stream`/`set_word_pos` accessors, splitting the 128-bit word position into two `u64` halves.
  No `rand_chacha` feature flags are needed. Pinned to `0.3.1` (unversioned in the plan's sketch;
  `Cargo.lock` already resolved it as a transitive dependency of `rand` 0.8.6).

- **Deviation: the `OnEnter(InRun)` guard covers the whole boot trio, not just `spawn_player`.** §5's
  plan text described "guarded 'spawn only if absent'" specifically for `spawn_player`. Implementing
  it surfaced a correctness hazard the plan's own risk table (§9) had flagged from the other
  direction: the golden campaign's scripted bot crosses in and out of `GameState::TalentPicker`
  several times over its 30 simulated seconds (via the XP-surge script), so `OnEnter(InRun)` fires
  repeatedly *inside the campaign itself* — a live, exercised, non-hypothetical case, not just an
  overlay-round-trip risk in the abstract. A guard scoped only to `spawn_player` would have let
  `generate_map`/`init_level_flow` re-fire on every one of those re-entries, silently rerolling the
  arena layout (burning extra `RunRng` draws) and resetting `LevelUpFlowState` (wiping band-pool
  progress) mid-campaign — a severe, hard-to-diagnose regression. The fix: all three systems
  (`spawn_player` in `PlayerPlugin`, `generate_map` in `WorldPlugin`, `init_level_flow` in
  `ProgressionPlugin`) share the identical run condition, `not(any_with_component::<Player>)`.
  Because `Commands` are deferred, every one of the three still sees "no player yet" on the true
  first boot (so all three run, exactly as before), and "a player already exists" on every later
  re-entry (real run-starts/restarts/resumes spawn their own fresh player *before* setting
  `NextState::InRun`; every overlay round-trip never despawns the player at all) — so the guard is
  correct without needing the risk table's documented fallback (an explicit transition pump in
  `Sim::new`).

- **Two pre-existing bugs found by new test coverage, both fixed in-phase (not deferred).**
  1. `enter_merchant` took a bare `Res<CurrentEncounter>`. It runs immediately after
     `handle_encounter_complete` in the same `.chain()`, and Bevy auto-inserts a sync point between
     ordered systems with a `Commands` dependency — so on the Act-3 boss clear,
     `handle_encounter_complete`'s `commands.remove_resource::<CurrentEncounter>()` had *already*
     applied by the time `enter_merchant` ran the same frame, failing parameter validation and
     panicking. No test exercised the Act-3-victory path before Phase 8's
     `tests/meta.rs::run_end_appends_a_scored_run_record_on_victory` — meaning **any real
     playthrough that reached the final boss would have crashed**. Fixed with `Option<Res<_>>`.
  2. `resume_run` replays `TalentAcquiredEvent`s onto a player it just respawned *the same frame*.
     `install_acquired_talent` needs that player's `AcquiredTalents`/`ActiveHooks` components to
     already exist, but `attach_talent_components` (the system that normally adds them) runs
     unordered relative to both — a real, not merely theoretical, race the first time any code path
     did "spawn a player and hand it talents in the same frame" (a fresh run never does this; only
     resume does). Fixed by having `resume_run` attach the components synchronously before replaying
     the events, and adding a `Without<AcquiredTalents>` guard to `attach_talent_components` so it
     can never later clobber what resume already installed.

- **Save-point semantics (§3.1), as literally specified.** The save snapshot is taken exactly where
  §3.1 names it — inside `handle_encounter_complete`, on its two non-terminal exits (regular →
  MapSelect, and a non-final act advance) — and nowhere else (not in `handle_map_select`'s branch
  pick). One consequence, already anticipated by §3.1's "reloaded fresh" wording but worth stating
  plainly: because `RunState.current_node` only advances when a branch is actually *picked* (in
  `handle_map_select`), a save taken at "just cleared node X, MapSelect is showing" still names node
  X as `current_node`. Resuming from that save therefore reloads node X **with a freshly-rerolled
  roster** (from the live RNG position), not a new node — a deliberate v1 simplification (the plan's
  own "standard for the genre, keeps the snapshot small"), not a bug. Confirmed as the intended
  reading by testing it directly: `tests/persistence.rs::resume_continues_the_rng_stream_exactly`
  proves the reroll is itself bit-exact, which is the property that actually matters for D1.

- **Score/menu key bindings (not specified by the plan, decided during implementation).** Main menu:
  `1` New Run, `2` Resume Run (only wired live when a save exists), `3` Scoreboard, `Esc` quit —
  extending Phase 7.5's existing `1`/`Esc` scheme rather than introducing new bindings. Login: **any**
  key advances (not just Enter), since there is nothing to choose on a single-profile splash.

- **Tests.** +4 `tests/persistence.rs`, +4 `tests/meta.rs`, +2 `tests/game_flow.rs`; new unit tests in
  `run/rng.rs` (RNG resume contract + ChaCha8 determinism), `run/state.rs` / `meta/state.rs` (RON
  round-trips), `meta/persistence.rs` (save-path resolution, corrupt/missing → default),
  `meta/score.rs` (the formula across act/node/level/victory/time). New `Sim` helpers: `run_state`,
  `meta`, `lock_hero`, `request_resume_run`, `enter_login`, `enemy_roster_signature`. Build
  warning-free throughout.

- **Presentation (never headless).** `ui/screens/login.rs` and `ui/screens/scoreboard.rs` are
  presentation-only, like every other screen; their logic (state transitions, the score formula, the
  unlock predicate) is exercised headless. Verified manually on the Windows build (WSL has no GPU).
