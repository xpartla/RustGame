# CLAUDE.md — RustGame

Bevy 0.16 roguelite (WoW-class-inspired), mid-way through a phased architecture migration.
This file is a map, not the content — the documents below are the source of truth. Read them
before changing gameplay code. (Ignore the parent directory's CLAUDE.md — that describes an
unrelated web project.)

## Where truth lives

| Document | Role |
|---|---|
| `Mechanics.md` | Game design: classes, ability kits, talents, acts/maps, user flow |
| `docs/architecture-plan.md` | Architecture + migration phases 0–9; **§8 amendments**; **§8.5 tech-debt register**; §8.6 Phase 4 delivered |
| `docs/phase3-plan.md` | Phase 3 plan + as-built notes (template for future phase plans) |
| `docs/phase4-plan.md` | Phase 4 plan + as-built notes (hero/stance system + Mage, focused vertical slice) |
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

The maintained register is **`docs/architecture-plan.md` §8.5** (Phase-4 outcomes in §8.6) —
each item has an owning phase. Highlights a future session must not "rediscover":

- ~~Library triplication → generic `DefLibrary<T>`~~ **DONE (Phase 4)** — `core/def_library.rs`;
  the three libraries are type aliases; add new def types via `register_def_library::<T>()`.
- `execute_ready_abilities` split — **not triggered in Phase 4** (the focused slice landed no
  code-driven hook); do it with the **first code-driven ability/status hook**.
- `suppress_abilities` is parsed but not resolved/consumed — Phase 5 must wire it.
- Projectile/status **visuals**: sprites + status tints **done (Phase 4)**; the **Blood Boil nova
  flash is still open** — needs a presentation-only cast-VFX bus (a logic-path spawn would move the
  golden baseline).
- Projectiles fly through walls — **accepted by the project owner (2026-07-05) for now**;
  revisit during Mage playtesting, not before.
- `resolved_cd > 0` guard ignores an Override(0) cooldown talent — fix with the first
  cooldown-manipulating talent.
- `HeroDef.base_stats` is data-only — per-hero HP/move-speed application is deferred (the Mage
  currently plays with the Death Knight's stats).

When you resolve a register item, update §8.5/§8.6 and the CHANGELOG in the same change.
