# Phase 7.5 Implementation Plan — UI Layer & Presentation Backlog

_Written 2026-07-05, while Phase 7 (act graph + encounters) is in flight. Companion to
`docs/architecture-plan.md` (§2 `ui/` module, §8.1(9) "UI phase missing entirely — new phase
between 7 and 8", §8.5 tech-debt register) and `docs/phase7-plan.md` §7 (deferrals that name
"the UI phase" as their revival trigger). This phase collects **every UI-and-presentation item
deferred without a home** and lands it between Phase 7 and Phase 8. As-built notes go in §9._

> **For the implementing agent:** read `docs/phase6-plan.md` / `docs/phase7-plan.md` first for the
> house style (compat-gate after every step; the golden master is the contract; declare behavior
> changes in the CHANGELOG). **This phase starts only after Phase 7 merges** — read phase7-plan §9
> (as-built) before 7.5D/7.5E/7.5F: several items here layer on Phase-7 structures
> (`CurrentEncounter`, `RunState`, `ActGraph`, the `MapSelect` keyboard picker, the Merchant node)
> and two items (per-encounter map re-render, any minimal MapSelect text overlay) may or may not
> have been delivered by Phase 7 — absorb what it left. The §0 decisions are **proposed defaults —
> confirm with the project owner before 7.5A**.

---

## 0. Proposed decisions (confirm with the owner before implementing)

| # | Decision | Recommendation & rationale |
|---|---|---|
| **D1 — Menu boot flow** | Does the windowed game keep Phase 7E's auto-start-run, or boot into a real main menu? | **Boot to Menu (recommended), windowed-only.** `GamePlugin` (not `GameLogicPlugin`) replaces its `auto_start_run` Startup system with `enter_main_menu` (sets `NextState(Menu)` at Startup; state applies before the first gated `Update`, so no InRun frame leaks). Flow: Menu → CharacterSelect → `start_run(seed, hero)`. The headless sim never sees it — `Sim::new_arena` stays in `InRun`, golden campaign untouched. This is the same isolation pattern as Phase 7's D1. The `game/state.rs` TODO's second half (move player/map spawning from `Startup` to `OnEnter(InRun)`) stays Phase 8 — the menu simply overlays the pre-spawned world. |
| **D2 — Merchant ops pulled forward** | `talent/systems/merchant.rs` ops (remove talent, 3-for-1 trade) are marked Phase 8; the merchant *overlay* is this phase. Build the overlay hollow, or pull the ops forward? | **Pull the ops forward (recommended).** The consumer side has been implemented since Phase 2 (`uninstall_removed_talent` handles `TalentRemovedEvent`, pops `ActiveHooks`); the trade-up reuses `generate_offer` with a rarity floor (exists) + the TalentPicker flow (exists). The ops are two small handlers; without them the merchant node is a hollow walk-through and the overlay has nothing to do. Phase 8 keeps only persistence-side merchant concerns (none known). |
| **D3 — Scoreboard** | Build the scoreboard screen now (in-memory `MetaState.run_history`) or wait for Phase 8? | **Defer to Phase 8 (recommended).** It needs run records (appended by run-end logic that Phase 8 owns), persistence to be meaningful across launches, and a **score formula that does not exist yet** (§8.1(10) — a design decision, not a UI task). The main menu ships the button greyed out; the screen is a natural Phase-8 deliverable alongside `RunRecord` writes. |
| **D4 — Input model** | Mouse-driven UI (bevy_ui `Interaction`) or keyboard-first? | **Keyboard-first everywhere (recommended); mouse as a windowed-only additive layer.** Every screen is drivable by keys (digits/arrows/Enter/Esc), with the input handled by **logic-side systems emitting events** — the established talent-picker pattern, and the only pattern that is headless-testable (bevy_ui's `Interaction` needs a window + layout pass and never runs in the sim). Mouse hover/click handlers may be added in presentation, but they must only emit the same logic events, never mutate state directly. |
| **D5 — Health-bar split** | The debug gizmo bars (`draw_health_bars`, PostUpdate) currently cover everyone. What does the HUD own? | **Player bar moves into the HUD; enemy gizmo bars stay; bosses get a HUD bar (recommended).** The HUD renders the player's health/XP/resource properly and `draw_health_bars` stops drawing the player's (enemy bars remain the cheap gizmo path until an art pass). A `MapBoss`/`ActBoss`-tagged enemy additionally gets a large top-of-screen HUD bar — the classic boss-fight readout, and `KillMapBoss` objectives need it to be legible. |

---

## 1. Scope

### 1.1 The deferred-item inventory this phase absorbs (with provenance)

| # | Item | Deferred by |
|---|---|---|
| 1 | **In-run HUD** — player health / XP+level / ability slots with cooldowns / stance / class-resource slot / objective tracker / boss bar | architecture §8.1(9); phase7-plan §7 ("HUD (health/cooldowns/XP/objective)") |
| 2 | **Game-over flow** — `GameState::GameOver` on death, death screen, restart | §8.1(9) ("player death is still a bare despawn"); `player/systems/death.rs` TODO("menu/UI epic") |
| 3 | **Pause** — Esc ⇄ `Paused` overlay with resume/quit + build summary | §8.1(9) ("GameState variants exist, transitions unwired") |
| 4 | **Main menu** — New Run / Resume (greyed) / Scoreboard (greyed) / Exit | §8.1(9); `game/state.rs` TODO(Phase 8) first half (D1) |
| 5 | **Character select** — hero cards from `HeroDef`, pick → start run | §8.1(9); phase4-plan §7 ("character-select UI — later phase"); phase7-plan §7 ("Phase 8") — pulled here, it's UI |
| 6 | **Visual act-graph map view** on `MapSelect` — node types + themes + reachability (Mechanics: "The player can see the encounter type, and the map theme") | §8.1(9); phase7-plan §7 ("the UI phase — Phase 7 ships only the keyboard picker") |
| 7 | **Merchant screen + ops** (remove / 3-for-1 trade) | §8.1(9); phase7-plan §7 + `talent/systems/merchant.rs` TODO(Phase 8) — pulled forward per D2 |
| 8 | **ThroneRoom curse banner** — show `RoomModifierDef.description` on entry (the field exists for exactly this) | implied by §6 Q1 ("see the threat" fantasy); no phase owned it |
| 9 | **Zone visuals** — translucent disc on `Added<PersistentZone>` | phase6-plan §7 ("a presentation pass") |
| 10 | **Cast-VFX bus + Blood Boil nova flash** — presentation-only event bus so logic never spawns VFX | §8.5 register ("needs a presentation-only cast-VFX event bus") |
| 11 | **Class-resource HUD slot** — generic bar shown when `ClassResource` is present (frost charges are Phase 9 *content*; the slot is UI) | phase4-plan §7 ("frost-charge resource + UI bar") — slot only |
| 12 | **Per-encounter map re-render** — floor/obstacle meshes rebuilt when the encounter's `TileMap` changes | phase7-plan §2.3 presentation note ("can trail the logic") — **absorb only if Phase 7 didn't deliver it; check §9 as-built** |

### 1.2 Explicitly NOT this phase (stays deferred, see §7)

Scoreboard screen + score formula (Phase 8, D3); "Resume Run" (Phase 8); hero lock/unlock greying
(needs persistent `MetaState` — Phase 8; all heroes render unlocked for now); "Log In" local
profile (Phase 8); Settings screen (nothing to configure — no audio/art yet); a separate "Heroes"
gallery (fold-into-character-select is enough for now); moving player/map spawn out of `Startup`
(Phase 8, per the `game/state.rs` TODO); damage numbers / minimap / tooltip polish (later art/UX
pass); frost-charge content (Phase 9).

---

## 2. Architecture

### 2.1 Ground rules (all inherited, all load-bearing)

1. **`ui/` reads, never owns** (architecture §2: "reads data from all other domains, owns
   nothing"). Screens render from logic resources/components; they hold no gameplay state.
2. **Input is logic-side.** Every interactive screen splits exactly like the talent picker:
   a logic system (in the owning domain — progression, run, talent, game) reads `ButtonInput` and
   emits events / mutates state; the `ui/screens/*` system only renders. This keeps every flow
   **headless-testable** — the sim drives the real input systems via `tap_key`.
3. **All rendering lives in `PresentationPlugin`.** The sim builds `GameLogicPlugin` only ⇒ every
   pure-UI step is **golden-master-neutral by construction**. The steps that do touch logic
   (death→GameOver, pause, menu flow, merchant ops, VFX-event emission) are individually gated
   and individually compat-checked.
4. **Gameplay events across overlays:** `Paused`, `TalentPicker`, `Merchant`, `MapSelect` are
   freeze states — `add_gameplay_event` already advances buffers only during `InRun`, so pending
   combat events survive any overlay. `GameOver` and `Menu` are terminal — buffers already clear
   on entry (`core/events.rs`). No changes needed; the new states just inherit the contract.
   Any NEW event this phase adds chooses deliberately (input-intent → `add_event`).

### 2.2 Module layout after this phase

```
src/ui/
  plugin.rs             ← registers every screen's spawn/render/despawn systems by state
  theme.rs              ← NEW: shared palette (rarity colors, panel/overlay styles), spawn
                           helpers (overlay root, title row, option row, bar widget).
                           talent_picker.rs is refactored onto it (visual-only change).
  screens/
    talent_picker.rs    ← existing; + rarity coloring via theme.rs
    hud.rs              ← NEW (7.5A): the whole in-run HUD
    game_over.rs        ← NEW (7.5B)
    pause.rs            ← NEW (7.5B)
    main_menu.rs        ← NEW (7.5C)
    character_select.rs ← NEW (7.5C)
    map_select.rs       ← NEW (7.5D): visual act-graph view
    merchant.rs         ← NEW (7.5E)
```

Logic-side additions (small, each in its owning domain):

```
player/systems/death.rs      ← death → NextState(GameOver) + GameOverSummary resource
game/  (or run/)             ← pause toggle system (Esc, InRun ⇄ Paused)
run/systems/reset.rs         ← NEW: teardown_run + restart (respawn player, fresh seed, start_run)
game/plugin.rs (GamePlugin)  ← enter_main_menu Startup (windowed-only, replaces auto_start_run)
hero/ (or run/)              ← StartRunRequest event + character-select input handler
talent/systems/merchant.rs   ← fill the two todo!() op handlers (D2)
ability/systems/execute.rs   ← emit CastVfxEvent (write-only in logic)
```

### 2.3 The in-run HUD (7.5A — pure presentation)

One `hud.rs` screen, spawned `OnEnter(InRun)`-once (or at startup, visible only in InRun),
updated by change-detection queries. Data sources are all existing logic state:

- **Health bar** — `Health` on the `Player` entity (current/max). `draw_health_bars` stops
  rendering the player (D5) — an equivalent-looking gizmo simply moves into real UI.
- **XP bar + level** — `Experience { current, to_next, level }`.
- **Ability slots** — query `AbilityInstance` entities with `owner == player`; slot labels from
  `HeroDef.stance_slots` for the *active* stance (LMB / RMB / Shift), plus a row for auto-cast
  passives (`AbilityDef.activation`). Cooldown fill from `AbilityCooldown.elapsed / duration`;
  a slot greys out when its `StanceGate` mismatches `ActiveStance` or `AbilitiesSuppressed`
  is present.
- **Stance indicator** — `ActiveStance` + `HeroIdentity` (hidden for non-stance heroes).
- **Class-resource bar** — renders only when `ClassResource` is present (empty slot today;
  frost charges light it up in Phase 9 with zero UI work).
- **Player status row** — `StatusEffectInstance`s targeting the player: name + remaining time.
- **Objective tracker** — `CurrentEncounter.objective` (Phase 7's `ObjectiveProgress`):
  KillAll → "Enemies left: N", Survive → countdown, KillMapBoss → boss name. Plus act/node
  ("Act 1 — Node 3/15") from `RunState`. **Hidden when `CurrentEncounter` is absent** (the
  arena/no-run world) — the HUD must never require a run to exist.
  _If `ObjectiveProgress` doesn't expose display counts (check Phase-7 as-built), add read-only
  fields to it — data additions, campaign-neutral._
- **Boss bar** — a top-center large bar for any living enemy tagged `MapBoss`/`ActBoss` (D5).

### 2.4 Game-over + pause (7.5B — the two logic-touching state flows)

**Death → GameOver.** `player_death` keeps the despawn (the `player_despawns_on_death` scenario
keeps passing) and additionally writes a `GameOverSummary` resource (level, act, node, hero —
captured *before* the entity is gone; `RunState`'s mirror fields may be mid-encounter-stale) and
sets `NextState(GameOver)`. Terminal-state event clearing is already wired. The death screen
renders the summary with two actions (logic-side input): **R — restart** (new seed, same hero) and
**M — main menu**. _Declared behavior change (CHANGELOG): death now freezes into GameOver instead
of leaving a dead world running; the campaign bot never dies ⇒ baseline unaffected._

**Restart / teardown** (`run/systems/reset.rs`) — the missing "run reset" primitive, which
Phase 8's Start-New-Run also needs: despawn run-scoped entities (`Enemy`, `Projectile`,
`PersistentZone`, `PickUp`, status-instance orphans, **and the dead player's `AbilityInstance`
entities** — nothing cleans those today), reset `LevelUpFlowState` / `RoomModifiers` /
`CurrentEncounter` / `RunState`, respawn the player from its `HeroDef` (the `Level1Granted`
deferred-grant path re-runs naturally on a fresh entity), reseed `RunRng`, call Phase 7's
`start_run`. Headless scenario: die → restart → a fresh deterministic run boots.

**Pause.** A logic system in `InRun` reads Esc → `Paused`; in `Paused` reads Esc → back. (Note:
Esc currently means "decline" in `TalentPicker` — no conflict, different states.) Gameplay is
already frozen by `in_state(InRun)` gating and in-flight combat events already survive (the
`freeze.rs` contract). The pause screen shows the **current build** — unlocked abilities +
`AcquiredTalents` with stack counts — which doubles as the playtester's build inspector.

### 2.5 Main menu + character select (7.5C)

- **Boot (D1):** `GamePlugin` swaps `auto_start_run` → `enter_main_menu`. Sim untouched.
- **Menu screen:** New Run (→ CharacterSelect), Resume Run (greyed, Phase 8), Scoreboard
  (greyed, Phase 8), Exit (sends `AppExit`). Digit/arrow keys, logic-side handler.
- **Character select:** one card per `HeroLibrary` entry (manifest order): `display_name`,
  stance pair or "no stance", resource model, level-1 ability display names resolved through
  `AbilityLibrary`. All heroes render unlocked until Phase-8 `MetaState` persistence. Pick →
  `StartRunRequest { hero_id }` → run reset (§2.4) + `start_run(seed, hero)`. Esc → Menu.
  This finally makes the Mage reachable without the debug M key (which stays, debug-only).
- `GameState` gains no new variants — `Menu`/`CharacterSelect`/`Paused`/`GameOver` all exist
  since Phase 0; this phase wires the transitions that were reserved.

### 2.6 Visual act-graph map view (7.5D — layers on Phase 7's MapSelect)

Replaces the presentation of Phase 7's minimal keyboard picker; **keeps its logic input contract**
(digit-key selection in `run/systems/select.rs`, or whatever §9 of phase7-plan says shipped).

- Renders `RunState.act_graph` as Slay-the-Spire columns: nodes positioned by column (derive by
  BFS depth from `entry` if Phase 7 didn't store a column index; if deriving is awkward, add a
  read-only `column` field to `EncounterNode` at generation — a neutral data addition).
- Per node: encounter-type glyph + label (Map/Boss/ActBoss/Throne/Merchant — text/shape only, no
  art assets), theme name, objective type for **reachable** nodes (Mechanics: the player sees
  encounter type + theme before choosing). Visited path dimmed, current node highlighted,
  reachable nodes numbered to match the selection keys.
- **ThroneRoom curse banner (item 8):** on entering a ThroneRoom encounter, show the node
  modifier's `display_name` + `description` (from `RoomModifierDef`) as a banner for a few
  seconds / until first input — pure presentation reading the `RoomModifiers` resource.

### 2.7 Merchant screen + ops (7.5E — D2)

In `GameState::Merchant` (entered on arriving at a Merchant node — coordinate with Phase 7's
as-built: it may auto-complete the node instead; rewire entry → `Merchant`, leave → the node
completes → `MapSelect`):

- Screen lists `AcquiredTalents` (name, rarity color, stack count) with three modes (logic-side
  state machine, digit/arrow keys): **Remove one talent**, **Trade 3-for-1**, **Leave**.
- Remove → `MerchantRemoveRequest` → the (new) handler emits `TalentRemovedEvent` — consumed by
  the already-implemented `uninstall_removed_talent`.
- Trade → select three → `MerchantTradeRequest` → handler emits 3× `TalentRemovedEvent` + calls
  `generate_offer` with min-rarity one step above the highest sacrificed (the rarity-floor
  machinery exists from the ThroneRoom kiss) → hands off to the TalentPicker flow.
- Scenarios drive requests headlessly; the golden campaign has no run ⇒ neutral.

### 2.8 Presentation backlog (7.5F)

- **Zone visuals (item 9):** `attach_zone_visuals` on `Added<PersistentZone>` — the established
  `attach_*_visuals` pattern (insert `Transform` + `Mesh2d` + translucent material on the logic
  entity itself, so despawn is automatic). Color by `zone_type` (D&D red / Consecrated gold /
  AMZ blue / Tree green), radius from `PersistentZone.radius`; `Follow` anchors already move the
  logic position, `sync_transform` does the rest.
- **Cast-VFX bus + nova flash (item 10):** `CastVfxEvent { caster, ability_id, origin, kind }`
  (plain `add_event` — presentation-consumed intent, fine to expire). `execute_ready_abilities`
  **writes** it (logic-side write-only: no state mutation, no RNG, no spawns ⇒ campaign trace
  unchanged — verify byte-identical). A presentation system spawns a fading nova ring for
  Blood Boil (radius from the resolved param carried in `kind`). **The existing logic-side cone
  flash path is left untouched** — migrating it onto the bus would delete logic-spawned entities
  and risk the baseline for zero behavior gain; it can migrate whenever a regen is otherwise
  scheduled. This closes the §8.5 nova-flash item.
- **Map re-render on encounter change (item 12):** only if Phase 7 deferred it — despawn old
  floor/obstacle meshes and re-run `render_map` on a `TileMap`-changed signal. Presentation-only.

---

## 3. File-level change map

| Area | File(s) | Change |
|---|---|---|
| UI core | `ui/theme.rs` (new), `ui/plugin.rs`, `ui/screens/mod.rs` | palette + spawn helpers; register all screens by state |
| HUD | `ui/screens/hud.rs` (new) | §2.3; `core/systems/debug.rs::draw_health_bars` skips the player (D5) |
| Game over | `player/systems/death.rs`, `ui/screens/game_over.rs` (new), `run/systems/reset.rs` (new) | GameOver transition + summary; death screen; teardown/restart |
| Pause | `game/` pause-toggle system (new, logic), `ui/screens/pause.rs` (new) | Esc toggle; build summary screen |
| Menu | `game/plugin.rs` (`GamePlugin` only), `ui/screens/main_menu.rs` (new) | `enter_main_menu` replaces `auto_start_run` (D1); menu screen + logic handler |
| Char select | `ui/screens/character_select.rs` (new), `hero/` or `run/` input handler + `StartRunRequest` | hero cards; pick → reset + `start_run` |
| Map view | `ui/screens/map_select.rs` (new), possibly `world/graph.rs` (`column` field) | §2.6 visual layer over Phase-7 picker; curse banner |
| Merchant | `talent/systems/merchant.rs` (fill todos), `ui/screens/merchant.rs` (new), `talent/plugin.rs` | ops handlers registered; overlay |
| VFX bus | `ability/systems/execute.rs`, `ability/` event def, `game/presentation.rs` + a `vfx.rs` | `CastVfxEvent` write; nova-flash consumer |
| Zones | `zone/systems/visuals.rs` (new), `game/presentation.rs` | `attach_zone_visuals` |
| Tests | `tests/game_flow.rs` (new), `tests/merchant.rs` (new), updates to `tests/combat.rs` | §6 scenarios |
| Docs | this §9; CHANGELOG "Phase 7.5"; architecture-plan §8.1(9)+§8.5+§8.10; testing.md; Mechanics.md user-flow notes; repo CLAUDE.md | 7.5G |

---

## 4. Content

No new RON content. (Hero cards, ability names, talent names, curse descriptions all come from
existing defs — `display_name`/`description` fields exist. If any def lacks a display name the
fix is a data edit, not schema work.) No new dependencies: bevy_ui ships with bevy 0.16's default
features and the talent picker already uses it — **no egui**, one UI stack.

---

## 5. Implementation sequence (each step independently `/compat-check`-able)

Ordered so pure-presentation work lands first (useful immediately for Windows playtesting of
Phase 7) and each logic-touching step is isolated. Confirm §0 first; verify Phase 7 merged.

**7.5A — Theme + HUD (presentation-only, neutral).** ★ `ui/theme.rs`; talent-picker refactor onto
it; `hud.rs` (all of §2.3); `draw_health_bars` player-skip. Zero logic changes ⇒ byte-identical.

**7.5B — GameOver + Pause (logic + screens).** ★ Death transition + `GameOverSummary`; pause
toggle; both screens; `run/systems/reset.rs` teardown/restart. Declared in CHANGELOG (death
behavior change; new Esc binding). Campaign bot never dies/pauses ⇒ expect byte-identical; the
`player_despawns_on_death` scenario gains a `game_state() == GameOver` assertion.

**7.5C — Main menu + character select (D1).** ★ `enter_main_menu` swap in `GamePlugin`;
menu + character-select screens with logic handlers; `StartRunRequest` → reset + `start_run`.
Windowed-only boot change (declared); sim path untouched ⇒ byte-identical. _(Verify on Windows:
boot → menu → pick Mage → Act-1 tutorial plays.)_

**7.5D — Act-graph map view + curse banner.** ★ `map_select.rs` over Phase 7's picker input;
column derivation (or neutral `column` field); ThroneRoom banner. Presentation-only ⇒ neutral.

**7.5E — Merchant overlay + ops (D2).** ★ Fill the two `merchant.rs` handlers; overlay; node
entry/exit wiring against Phase-7's merchant flow. Ops run only in `Merchant` state ⇒ campaign
neutral; scenarios cover both ops.

**7.5F — Presentation backlog.** ★ Zone discs; `CastVfxEvent` + nova flash (**the one logic-file
edit here — verify byte-identical with extra care**, it touches `execute_ready_abilities`); map
re-render if Phase 7 left it. Closes §8.5 nova item + phase6 zone-visuals deferral.

**7.5G — Docs + final gate.** §9 as-built; CHANGELOG "Phase 7.5"; architecture-plan: close
§8.1(9), update §8.5 (nova), add §8.10 delivered-summary, §7 marker; testing.md scenario list;
Mechanics.md user-flow annotations (which screens are live); repo CLAUDE.md map. Full
`/compat-check`.

---

## 6. Validation & testing suite

### 6.1 Headless scenarios (the UI never runs in the sim — its *logic* does)

New `tests/game_flow.rs`:
1. **player_death_enters_game_over** — kill the player → state is `GameOver`, summary resource
   captured, pending gameplay events cleared (extends the existing despawn test).
2. **restart_after_death_boots_a_fresh_run** — die → restart request → run-scoped entities gone
   (incl. the dead player's `AbilityInstance`s), fresh player at level 1, `start_run` loaded the
   entry encounter, deterministic under a fixed seed.
3. **esc_toggles_pause_and_preserves_combat_events** — write a `DamageEvent`, open `Paused` the
   same frame, resume → the damage resolves (mirrors `freeze.rs`, now for `Paused`).
4. **pause_does_not_tick_the_world** — positions/cooldowns/status timers frozen across N paused
   frames.
5. **character_select_starts_the_chosen_hero** — drive Menu → CharacterSelect → pick Mage →
   `HeroIdentity == mage`, level-1 grant ran, encounter live. (States are logic; fully sim-able.)

New `tests/merchant.rs` (7.5E):
6. **merchant_remove_uninstalls_talent_and_hook** — acquire a Behavior talent, remove it →
   `AcquiredTalents` count drops, `ActiveHooks` popped.
7. **merchant_trade_offers_higher_rarity** — sacrifice 3 commons → offer contains only
   Rare-or-above (floor mechanics shared with the ThroneRoom kiss).

### 6.2 Golden master

**Byte-identical at every ★, no regeneration.** The campaign never dies, pauses, opens a menu, or
visits a merchant, and every screen lives in `PresentationPlugin`. The two logic touchpoints with
InRun reach (death-system edit, `CastVfxEvent` emission) mutate nothing on the campaign path. Any
diff = a leaked gate or an accidental state mutation — fix it, don't regenerate.

### 6.3 Windows manual verification checklist (visuals can't be asserted headless)

Boot→menu→select→run loop; HUD (bars fill/drain, cooldown sweeps, stance flips on Q, objective
counts down, boss bar on a warlord); map view (reachable nodes numbered, themes shown); throne
banner; pause build list; death screen + restart; merchant flows; zone discs under D&D /
Consecrated / AMZ; Blood Boil nova flash. Screenshot pass into the phase §9 notes.

---

## 7. Deferred — with the trigger that revives each

| Deferred | Revived by |
|---|---|
| Scoreboard screen + **score formula** + run-record writes (D3) | Phase 8 (persistence gives it data; formula is a §8.1(10) design decision) |
| "Resume Run" enablement | Phase 8 (RunState serialization) |
| Hero locked/unlocked greying on character select | Phase 8 (persistent `MetaState`) |
| "Log In" local profile screen | Phase 8 |
| Move player/map spawn from `Startup` to `OnEnter(InRun)` | Phase 8 (with Resume, per the `game/state.rs` TODO) |
| Settings screen | when there's something to set (audio/keybinds — out of scope until further notice) |
| Separate "Heroes" gallery screen | Phase 9 content pass (character select covers it) |
| Migrating the existing cone-flash to the VFX bus | the next deliberate baseline regen (zero behavior gain alone) |
| **Mouse-input handlers** (hover/click) for every screen (D4 — keyboard-first shipped; mouse is additive, presentation-only, untestable headless) | a later UX pass (must only emit the same logic events the keys do) |
| Damage numbers, minimap, tooltips, gamepad, art/audio | later UX/art pass |
| Frost-charge bar *content* | Phase 9 Mage capstone (the HUD slot is ready) |
| **Orphaned `AbilityInstance` cleanup on enemy death** (found this phase; filed to architecture-plan §8.5) | the next enemy/perf pass — the run-reset already despawns them, so restarts are clean |

---

## 8. Risks & mitigations

| Risk | Mitigation |
|---|---|
| A "presentation" system accidentally mutates logic (classic UI creep) | Rule 2.1(1)/(2): screens query read-only; all input handlers live logic-side and are individually reviewed. `/compat-check` at every ★. |
| Death→GameOver breaks scenarios that kill the player | Only `player_despawns_on_death` does; it's updated in the same step and the change is CHANGELOG-declared. Terminal event-clearing already exists. |
| `CastVfxEvent` emission in `execute_ready_abilities` perturbs the campaign | Write-only, no RNG, no spawns, plain `add_event`. Verify byte-identical in 7.5F specifically. If it drifts, emit from a post-apply observer instead. |
| Menu boot leaks into the sim | The swap happens in `GamePlugin` only (the Phase-7E auto-start slot). `Sim` builds `GameLogicPlugin` — it can't see it. Scenario 6.1(5) drives menus explicitly. |
| Phase-7 as-built diverges from this plan's assumptions (picker shape, merchant node flow, `ObjectiveProgress` fields, map re-render) | The 7.5D/E/F steps each start by reading phase7-plan §9; this plan marks every such dependency inline. Adjust wiring, not architecture. |
| Restart/teardown misses an entity class → state bleeds between runs | `reset.rs` reuses Phase 7's encounter-teardown list + player-owned extras; scenario 6.1(2) asserts a clean world census after restart. |
| bevy_ui layout differs across window sizes | Percent/flex layouts (the talent-picker pattern), no absolute pixel positioning except HUD edge anchors. |

---

## 9. As-built notes (completed 2026-07-05)

Phase 7.5 landed as planned across 7.5A–7.5G, at **full scope**. **The golden master moved zero times —
byte-identical at every step, no regeneration** — matching Phases 4–7. The whole UI lives in
`PresentationPlugin` (which the headless sim never builds), and every logic touchpoint is inert on the
campaign path. The §0 decisions were all confirmed with the owner as the recommended defaults.

- **§0 decisions (resolved).** **D1** boot to a real main menu (windowed-only: `GamePlugin` swaps
  `auto_start_run` → `enter_main_menu`). **D2** merchant ops pulled forward (remove + 3-for-1 trade).
  **D3** scoreboard deferred to Phase 8 (menu button greyed). **D4** keyboard-first everywhere (logic-side
  input systems; no mouse handlers this phase). **D5** player health/XP moved into the HUD, enemy gizmo
  bars stay, bosses get a top-center HUD bar.

- **What Phase 7 had already delivered (absorbed, not re-built).** Two inventory items were already done
  by Phase 7: **item 12 (per-encounter map re-render)** — `rerender_map` was live in `PresentationPlugin`
  — and a **minimal `MapSelect` text overlay**. So 7.5D became a *visual upgrade* of the existing overlay
  (keeping Phase 7's `handle_map_select` input contract), not new plumbing, and item 12 needed no work.
  `EncounterNode.column` already existed, so the map view needed no data addition. The act boss is a
  `warlord` tagged `MapBoss` (there is no separate `ActBoss` marker), so the boss bar keys off `MapBoss`
  alone.

- **Golden-master neutrality held (the load-bearing constraint).** The headless sim builds
  `GameLogicPlugin` only; every screen is registered under `PresentationPlugin`, so no UI runs headless.
  The logic touchpoints are individually inert in the campaign: death→GameOver never fires (the bot never
  dies), the Esc pause is gated on an Esc press the bot never sends, the merchant systems gate on their
  state / request events the campaign never reaches, the menu boot is windowed-only, and the cast-VFX
  write is write-only (no state/RNG/spawn) so it does not move the trace even though the campaign casts
  Blood Boil through it. Verified byte-identical after every step; the cast-VFX write (7.5F, the one
  campaign-reachable logic edit) was verified with extra care.

- **Run-reset placement (7.5B).** The reset is an exclusive `&mut World` fn (`reset_and_start_run`) that
  reuses the real Startup systems via `run_system_once` (`spawn_player`, `init_level_flow`) so a restart
  reproduces the boot path exactly. Both the death screen's R and character-select route through one
  `StartRunRequest` event → `apply_start_run_request` (gated `on_event`, so it never runs — or perturbs —
  the campaign). Teardown despawns every run-scoped entity class **including the separate
  `AbilityInstance` entities** (nothing else cleans a dead player's or a despawned enemy's), asserted by a
  captured-entity census in `restart_after_death_boots_a_fresh_run`.

- **Merchant flow (7.5E).** Phase 7's Merchant node auto-completed as a `Rest` objective; 7.5E rewired it:
  `ObjectiveProgress::Rest` no longer completes via the objective path, `enter_merchant` opens
  `GameState::Merchant` once the empty room loads, and the shop is left **directly to MapSelect** (not
  back through InRun) so `enter_merchant` cannot re-fire and race the completion. The trade-up reuses the
  ThroneRoom-kiss picker via a new `TradeUpRewardEvent` (defined in `talent`, consumed in `progression` —
  keeping the dependency one-directional, `progression → talent`).

- **Deviations from the plan (small).**
  - The curse banner reads `CurrentEncounter.modifier` → `RoomModifierDef` (name + description), not the
    `RoomModifiers` resource (which holds only the stat modifiers, no text). It lives in the HUD module so
    it is torn down with the HUD on `OnExit(InRun)`.
  - The nova flash is drawn with **gizmos** (an expanding, fading ring), mirroring the existing debug
    hitbox-gizmo path — no mesh assets, and it despawns itself on a timer. The bus (`CastVfxEvent`) carries
    a `CastVfxKind` (`Nova{radius}` / `Other`); only `Nova` is consumed for now (other casts keep their
    existing gizmo VFX).
  - HUD internals (marker components + update systems) are all private to `hud.rs`; the module exposes a
    single `pub fn plugin(app)` so it leaks no internal types into the crate API.
  - `NextState` transitions apply on the *next* frame, so the sim helpers/tests step once after a key tap
    before asserting the new state (documented in the pause/menu tests).

- **Presentation (never headless, does not gate the golden master).** HUD, all overlays (menu, character
  select, game-over, pause, merchant, the visual map view), the curse banner, zone discs, and the nova
  flash are verified manually on the Windows build (WSL has no GPU). Only their *logic* (state flows,
  reset, ops, VFX-event emission) is asserted headless.

- **Tests: 136 passing** (was 129). +5 golden scenarios in `tests/game_flow.rs` (death→GameOver;
  restart→fresh deterministic run with a clean census; Esc-pause preserves in-flight combat events; pause
  freezes the world; character-select starts the chosen hero) and +2 in `tests/merchant.rs` (remove
  uninstalls talent + hook; trade offers a higher rarity). `tests/combat.rs::player_despawns_on_death`
  gained a `GameOver` assertion. Build warning-free.

- **Debt updates (architecture-plan §8.1(9)/§8.5/§8.10).** §8.1(9) "UI phase" is **closed** except the
  Phase-8 carve-outs (scoreboard + score formula, Resume Run, hero unlock greying, Log-In profile, moving
  player/map spawn out of `Startup`). §8.5's **Blood Boil nova-flash** row is **resolved** (the cast-VFX
  bus). `HeroDef.base_stats` per-hero application remains the last open §8.5 row (deferred — the Mage
  still plays with the DK's HP/speed). New §8.10 records the delivered summary.
