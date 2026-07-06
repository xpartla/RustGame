# CLAUDE.md — RustGame

Bevy 0.16 roguelite (WoW-class-inspired), mid-way through a phased architecture migration.
This file is a map, not the content — the documents below are the source of truth. Read them
before changing gameplay code. (Ignore the parent directory's CLAUDE.md — that describes an
unrelated web project.)

## Where truth lives

| Document | Role |
|---|---|
| `Mechanics.md` | Game design: classes, ability kits, talents, acts/maps, user flow |
| `docs/architecture-plan.md` | Architecture + migration phases 0–9; **§8 amendments**; **§8.5 tech-debt register**; §8.6 Phase 4 delivered; §8.7 Phase 5 delivered; §8.8 Phase 6 delivered; §8.9 Phase 7 delivered; §8.10 Phase 7.5 delivered; §8.11 Phase 8 delivered |
| `docs/phase3-plan.md` | Phase 3 plan + as-built notes (template for future phase plans) |
| `docs/phase4-plan.md` | Phase 4 plan + as-built notes (hero/stance system + Mage, focused vertical slice) |
| `docs/phase5-plan.md` | Phase 5 plan + as-built notes (enemy abilities + AI + faction-aware engine) |
| `docs/phase6-plan.md` | Phase 6 plan + as-built notes (persistent zones + code-driven ability hooks) |
| `docs/phase7-plan.md` | Phase 7 plan + as-built notes (act graph + room / encounter system) |
| `docs/phase7.5-ui-plan.md` | Phase 7.5 plan + as-built notes (UI layer: HUD, menus, game-over/pause, map view, merchant, VFX bus) |
| `docs/phase8-plan.md` | Phase 8 plan + as-built notes (persistence + meta: RunRng → ChaCha8, RunState/MetaState serde, save/resume, scoreboard, Log-In) |
| `CHANGELOG.md` | **The behavior contract** (see below) |
| `docs/testing.md` | Headless harness, golden scenarios/baseline, regeneration procedure |

## Non-negotiable contracts

- **Every behavior change must be declared in CHANGELOG.md.** Anything the test ladder catches
  that is not declared is a regression. Run `/compat-check` after finishing a phase or before
  committing.
- **Golden baseline** (`tests/golden/campaign_baseline.ron`) regenerates only via
  `UPDATE_GOLDEN=1 cargo test --test golden_campaign`, only for CHANGELOG-declared changes,
  committed together with the change. Never regenerate around nondeterminism
  (`campaign_is_reproducible_within_a_build` failing = defect to fix).
- **Build is warning-free.** Any new `cargo check` warning is a finding.
- **Scheduling contracts** (docs/testing.md): the frame skeleton is
  `MovementSet::Intent → Integrate → CombatSet::Damage → Apply → StatusSet::Tick →
  CrossInteract → CombatSet::Death`; combat-resolution events use `add_gameplay_event`
  (survive overlay freezes), input-intent events use plain `add_event`.
- **Logic/presentation split**: gameplay code must never read presentation components
  (`EnemyAppearance`, meshes); logic collision uses `Hurtbox`. New logic spawns carry data
  components; presentation dresses them on `Added<T>` (`src/game/presentation.rs`).

## Environment constraints

- **WSL has no GPU** — the windowed game cannot run here. All testing is headless via
  `src/sim/` (`Sim::new_arena`); visuals are verified manually on Windows.
- Nothing is committed by agents unless the user asks; the user commits.

## Known tech debt (before you add to it)

The maintained register is **`docs/architecture-plan.md` §8.5** (Phase-4 outcomes in §8.6, Phase-5
in §8.7, Phase-6 in §8.8, Phase-7 in §8.9, Phase-7.5 in §8.10, Phase-8 in §8.11) — each item has an
owning phase. Highlights a future session must not "rediscover":

- ~~Library triplication → generic `DefLibrary<T>`~~ **DONE (Phase 4)** — `core/def_library.rs`;
  add new def types via `register_def_library::<T>()` (`EnemyDef` joined the same way in Phase 5).
- ~~`suppress_abilities` parsed but not consumed~~ **DONE (Phase 5)** — `AbilitiesSuppressed` marker,
  folded by `resolve_actor_status`, gates auto-cast/execute + hero input/stance.
- ~~Enemy ability/AI framework; enemy scaling; enemy projectiles~~ **DONE (Phase 5, §8.7)** — data
  `EnemyDef` + faction-aware engine + `contact_melee`/ranged caster + data-only scaling. The **AI
  "registry" is a component enum** (`AiBehavior`), not the scaffold trait; the scaffold
  `enemy/behavior.rs` is deleted.
- ~~Act graph + themed encounters; `ThemeDef`/theme spawning; `Elite`/boss spawn roles; live enemy
  scaling driver~~ **DONE (Phase 7, §8.9)** — seeded `build_act_graph` + per-room `world/generator.rs`
  + the live `run` module (`RunState`/`CurrentEncounter`/`RunPlugin`); a themed depth-scaled encounter
  spawner drives the Phase-5 curve; `MapBoss` spawn roles; ThroneRoom curse (`RoomModifiers` →
  `extra_modifiers` for Hostile casts) + Rare-floor kiss; Merchant rest node; a minimal
  `GameState::MapSelect` keyboard picker. Still open: **multi-phase boss AI + enemy DoT kits + the real
  per-theme rosters (Phase 9)**; RunState save/resume (Phase 8); the player-stat ThroneRoom curses'
  bespoke consumers. (Merchant ops + the visual act-graph map view landed in Phase 7.5, §8.10.)
- ~~UI layer: HUD, menus, character select, game-over/pause, visual map view, merchant screen~~
  **DONE (Phase 7.5, §8.10)** — full UI surface: `ui/theme.rs` + `ui/screens/*` (hud, main_menu,
  character_select, game_over, pause, map_select visual view, merchant); death → `GameState::GameOver`
  + `run/systems/reset.rs` restart; windowed boot Menu → CharacterSelect → run (`enter_main_menu`
  replaced `auto_start_run`); merchant remove/trade ops; zone discs + the cast-VFX bus (nova flash).
  Every screen is presentation-only (verified on Windows); its logic is headless-tested.
- ~~Persistence: RunState save/resume; MetaState (hero unlocks + scoreboard + score formula);
  Log-In; player/map spawn `Startup`→`OnEnter(InRun)`~~ **DONE (Phase 8, §8.11)** — `RunRng` switched
  to `rand_chacha::ChaCha8Rng` (hand-serialized; the phase's one declared golden-master regen);
  `run/systems/persistence.rs` (`sync_run_state`/`tick_run_timer`/`save_run_snapshot`/
  `record_run_end`/`resume_run`); `meta/persistence.rs` (pure serialize/deserialize + a
  windowed-only disk layer, sim never touches a filesystem); `ui/screens/login.rs` +
  `ui/screens/scoreboard.rs`; `GameState::Login`/`Scoreboard`. Two pre-existing bugs surfaced and
  fixed along the way: `enter_merchant`'s bare `Res<CurrentEncounter>` panicked on the Act-3 victory
  path (no test had ever reached it before); a same-frame talent re-install onto a resumed player
  could race `attach_talent_components` (fixed with a synchronous attach + a `Without<AcquiredTalents>`
  guard). Also resolves the orphaned-`AbilityInstance` leak (§8.5) — `enemy_death` and
  `despawn_encounter_entities` now reap an enemy's owned instances.
- ~~Persistent zones + AMZ projectile-blocking~~ **DONE (Phase 6, §8.8)** — `zone` module live
  (`dropped_zone` + `PlayerZonePresence` + occupant DoT/regen + AMZ blocking). New zone abilities via
  `AbilityDef.zone: Option<ZoneSpec>`. Deferred to Phase 9: cross-ability zone buffs, Tree Conduit's
  enhanced-attack consumer, the AMZ-follow talent.
- ~~`execute_ready_abilities` split (do it with the first code-driven hook)~~ **DONE (Phase 6)** —
  `ability/hooks.rs` (`HookRegistry`/`AbilityHook`); execute interleaves Pre/Post hooks gated on
  `ActiveHooks` + registration. First hook: `blood_boil_dnd_range`. `bone_shield` stays inert until
  the shield/absorb system (§8.1(5)) lands — its Post-hook plumbing now exists.
- ~~Projectile/status **visuals** + the Blood Boil nova flash~~ **DONE (Phase 4 + 7.5)** — sprites +
  status tints (Phase 4); the nova flash landed Phase 7.5 via the **cast-VFX bus** (`CastVfxEvent`,
  write-only from `execute_ready_abilities` → drawn by `game/vfx.rs`), plus zone discs. The logic-side
  cone flash stays on gizmos (migrating it earns nothing, risks the baseline).
- Projectiles fly through walls — **accepted by the project owner (2026-07-05) for now**;
  revisit during Mage playtesting, not before.
- `resolved_cd > 0` guard ignores an Override(0) cooldown talent — fix with the first
  cooldown-manipulating talent.
- `HeroDef.base_stats` is data-only — per-hero HP/move-speed application is deferred (the Mage
  currently plays with the Death Knight's stats). Enemy `base_stats`, by contrast, ARE applied
  (Phase 5) and the enemy `scaling` curve is now driven live by encounter depth (Phase 7, §8.9).
  **This is now the last open §8.5 row** (deferred out of Phase 8 by D4-OUT — applying it is a
  second golden regen + a balance call, → Phase 9).

When you resolve a register item, update §8.5/§8.6/§8.7/§8.8/§8.9/§8.10/§8.11 and the CHANGELOG in
the same change.
