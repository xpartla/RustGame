# Phase 6 Implementation Plan — Persistent Zones + Code-Driven Ability Hooks

_Written 2026-07-05 against `main` @ `dad7558` (Phases 0–5 + testing infra complete).
Companion to `docs/architecture-plan.md` (§3.6 zones, §3.3/§3.4 ability/talent hooks, §4 worked
example, §7 phase plan, §8.5 debt register / §8.7 Phase-5 outcomes) and `docs/testing.md`.
As-built notes go in §9._

---

## 0. Decisions locked for this phase

Two consequential decisions were resolved with the project owner before planning. They set Phase 6
wider than architecture-plan §7's three bullets (which describe only the "focused" zone core).

| # | Decision | Consequence |
|---|---|---|
| **D1** | **Build the code-driven ability-hook system now.** The first zone-conditioned talent ("Blood Boil has double range inside D&D") is implemented as a real `AbilityHook` (the architecture-plan §3.4/§4 design), **not** a declarative `ConditionalModifier`. This finally lands the `HookRegistry` + `ActiveHook` consumption and the `execute_ready_abilities` **resolve/apply split** that §8.5 has parked since Phase 3 for "the first code-driven hook." | The engine gains a general pre/post hook extension point (unblocks every future behavior-rewriting talent). `execute_ready_abilities` is refactored around the hook points. The split **must stay byte-identical** (§6). |
| **D2** | **Full zone scope, including occupant-tick effects and the AMZ projectile-blocking zone.** Not just the §7 core (zone entities + presence + D&D/Tree-Conduit emitters + the one validation talent) — also a **generic zone-occupant tick** (damage the opposing faction inside a zone; regen the owner inside) and the **AMZ** friendly zone that destroys enemy projectiles entering it (the §8.7 "Phase 6+" item). | Zones become *active* (Consecrated Ground DoT + D&D regen go live), and the projectile engine learns zone interaction. First zone that mutates combat every tick; first projectile-destruction mechanic. |

**Schema decision (owner-informed, not a blocking question).** `zone_type` cannot live in
`base_params` (that map is `f32`-only — the same limitation Phase 3 hit and solved with the
declarative `effects` list). Phase 6 adds a typed `zone: Option<ZoneSpec>` field to `AbilityDef`
(zone type string + anchor kind + `blocks_projectiles` flag; radius/duration/dps/regen stay
numeric params so the talent modifier stack still reaches them). The `dropped_zone` behavior
returns a zone-spawn request in `CastOutcome` and the execute system spawns the entity from
`def.zone` + resolved params — **exactly** the pattern `projectile` already uses (`CastOutcome.projectile`
+ execute spawns the entity with baked effects). No new machinery, one consistent shape.

---

## 1. Scope

### In scope
1. **Zone core (wire the scaffold).** `PersistentZone` / `ZoneAnchor` / `PlayerZonePresence` (all
   already written in `src/zone/`, `plugin.rs` is `todo!`) go live: `pub mod zone;` in `lib.rs`,
   `ZonePlugin` into `GameLogicPlugin`, presence rebuilt each frame before combat, lifetime +
   follow-anchor maintenance.
2. **Zone-emitting abilities.** A `dropped_zone` behavior + the `AbilityDef.zone: Option<ZoneSpec>`
   schema; execute spawns a `PersistentZone` carrying the caster's `Faction` and baked effect
   numbers. **D&D** (existing L1 DK ability, `behavior: "dropped_zone"`, currently inert) goes live;
   **Tree Conduit** ships as a second (marker-only) zone-type demonstrator (per §7).
3. **Code-driven hook system (D1).** `AbilityHook` trait + `HookRegistry` resource + a `HookContext`;
   `execute_ready_abilities` split into resolve → **Pre hooks** → behavior/apply → **Post hooks**,
   each hook firing only if the caster carries the matching `ActiveHook` (installed on talent
   acquisition since Phase 2, never consumed until now).
4. **The validation talent (testing.md Phase-6 DoD).** `blood_boil_dnd_range` — a Pre hook that reads
   `PlayerZonePresence` and doubles Blood Boil's `radius` when the caster stands in `death_and_decay`.
   The `blood_boil_dnd_range_rare.talent.ron` already exists (a `Behavior` effect, in the MANIFEST);
   this phase makes it *do* something.
5. **Zone occupant-tick effects (D2).** A generic `ZoneEffects` component + `zone_tick_effects` system:
   per tick, damage opposing-faction actors inside (Consecrated Ground DoT) and regen the owner inside
   (D&D healing). Neutral where no zone exists.
6. **AMZ projectile-blocking (D2).** `amz.ability.ron` (auto-cast, `blocks_projectiles: true`) + a
   `block_projectiles_in_zones` system that destroys projectiles targeting the zone's faction while
   inside it — with the Mechanics exception "no effect if emitted from inside."
7. **Full test suite + docs.**

### Out of scope (explicitly deferred — see §7)
Cross-ability zone buffs (Death Strike bonus damage inside D&D; Heart Strike +1 target inside — need
Heart Strike + a Death-Strike zone hook, Phase 9 content); Tree Conduit's "enhanced next animal
attack" consumer (Druid enhanced-attack state machine, Phase 9); the AMZ **epic "attached to you"**
talent (the `FollowCaster` anchor *mechanism* is built + tested this phase; the talent that flips a
base AMZ to follow is content); the **bone-shield Post hook** implementation (needs the shield/absorb
system, §8.1(5) — its `ActiveHook` plumbing rides along free but the hook stays unregistered → inert);
Consecrated Ground as a real Paladin ability (no Paladin hero yet — ships as a demonstrator, like the
Phase-3 Fireblast/Frostbolt/Scratch); **zone visuals** (a presentation-only translucent disc, deferred
to a presentation pass); projectiles still pass through **walls** (owner-accepted 2026-07-05).

---

## 2. Architecture

### 2.1 Zone core — wiring the existing scaffold

`src/zone/` already contains a complete `components.rs` (`PersistentZone`, `ZoneAnchor`,
`PlayerZonePresence` with `is_inside`) and working `systems/lifetime.rs`
(`tick_zone_lifetimes`, `move_anchored_zones`) + `systems/presence.rs`
(`build_player_zone_presence`). Only `plugin.rs` is a `todo!()` and `zone` is absent from `lib.rs`.

```
// lib.rs: add `pub mod zone;` (joins the crate like hero did in Phase 4).
// ZonePlugin::build:
//   app.init_resource::<PlayerZonePresence>()
//   Update, InRun-gated:
//     (tick_zone_lifetimes, move_anchored_zones, build_player_zone_presence)
//        .chain().in_set(MovementSet::Integrate).after(world_to_grid)
```

Placing zone maintenance at the **end of `MovementSet::Integrate`** (after `apply_velocity` →
`world_to_grid`) means: positions are already settled, expired zones are reaped, follow-anchors have
caught up, and `PlayerZonePresence` is fresh **before** `CombatSet::Damage` — where ability execution
and zone-tick damage read it. This respects the Phase-3.1 movement pin (positions are not perturbed;
zone systems never write an actor's `WorldPosition`), so it is baseline-neutral by construction: with
zero zones alive (the campaign never casts one — see §6) every zone system is an empty-loop no-op.

`PersistentZone` gains no fields, but each zone entity is spawned **carrying a `Faction`** (the
caster's, baked at spawn, mirroring `ProjectilePayload.target_faction`). Occupant damage hits the
*opposing* faction; regen heals the owner. `build_player_zone_presence` stays player-centric (it feeds
player-cast gating like the Blood Boil talent); occupant queries in §2.5 hit the world directly.

### 2.2 Zone-emitting abilities (`dropped_zone` + `ZoneSpec`)

```rust
// ability/assets.rs — AbilityDef gains:
#[serde(default)] pub zone: Option<ZoneSpec>,

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ZoneSpec {
    pub zone_type: String,                      // ZoneTypeId (→ PlayerZonePresence key)
    #[serde(default)] pub anchor: ZoneAnchorKind, // Fixed (default) | FollowCaster
    #[serde(default)] pub blocks_projectiles: bool, // AMZ (§2.6)
}
#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
pub enum ZoneAnchorKind { #[default] Fixed, FollowCaster }
```

```rust
// ability/behavior.rs — new behavior + CastOutcome field:
pub struct ZoneSpawn { pub center: Vec2 }     // + CastOutcome { .. , zone: Option<ZoneSpawn> }

pub struct DroppedZone;
impl AbilityBehavior for DroppedZone {
    fn resolve(&self, ctx, _params) -> CastOutcome {
        CastOutcome { origin: ctx.origin, zone: Some(ZoneSpawn { center: ctx.origin }), ..default() }
    }
    fn needs_aim(&self) -> bool { false }      // drops at the caster; no aim
}
```

**Execute** (`execute_ready_abilities`), after `resolve_effects`, if `outcome.zone` is `Some` and
`def.zone` is `Some`, spawns the zone from the resolved params:

```
radius   = params.get("zone_radius")
duration = params.get("zone_duration")
anchor   = Fixed(center) | Follow(caster)          // from ZoneSpec.anchor
faction  = owner_faction (already in the owners query)
commands.spawn((
    PersistentZone { zone_type: spec.zone_type, owner: caster, radius, duration: Timer(duration), anchor },
    WorldPosition(center),
    faction,
    // only when the ability defines occupant effects (§2.5) / blocking (§2.6):
    ZoneEffects { damage_per_second: params.get("damage_per_second"),
                  regen_fraction:    params.get("regen_percent_per_second") / 100.0,
                  tick: Timer::from_seconds(ZONE_TICK_INTERVAL, Repeating) },   // if either > 0
    ZoneBlocksProjectiles,                                                       // if spec.blocks_projectiles
));
```

D&D wiring: `dnd.ability.ron` grows a `zone: (zone_type: "death_and_decay", anchor: Fixed)` block,
keeps `activation: Input` (it is the RMB **Special** — Mechanics: "D&D — Special Attack"), and its
`damage_per_second` is set to **0** (per Mechanics, D&D is a *buff* zone, not a damage zone — only
`regen_percent_per_second` is live; the `damage_per_second: 2.0` scaffold value is dropped). Register
`dropped_zone` in `BehaviorRegistry`. Because D&D stays `Input` and the golden bot never presses RMB
nor triggers `"dnd"`, **no D&D zone ever spawns in the campaign** ⇒ neutral (§6).

### 2.3 Code-driven ability hooks (D1 — the load-bearing addition)

The talent module has maintained `ActiveHooks` on the player since Phase 2 (`install_acquired_talent`
pushes a `Behavior` talent's `HookId`) but nothing has ever consumed it. Phase 6 adds the consumer,
mirroring `BehaviorRegistry` exactly:

```rust
// ability/hooks.rs (new)
pub struct HookContext<'a> {
    pub caster: Entity,
    pub zones: &'a PlayerZonePresence,     // what the first hook needs; grows as hooks demand
}
pub trait AbilityHook: Send + Sync + 'static {
    fn pre(&self, _ctx: &HookContext, _params: &mut ResolvedParams) {}   // mutate params before resolve
    fn post(&self, _ctx: &HookContext, _outcome: &CastOutcome) {}        // react after apply
}
#[derive(Resource, Default)]
pub struct HookRegistry { hooks: HashMap<HookId, Box<dyn AbilityHook>> }  // register/get like BehaviorRegistry
```

`ResolvedParams` gains a mutator (`set(stat, v)` / `scale(stat, factor)`) so a Pre hook can rewrite a
resolved number.

**Execute split.** `execute_ready_abilities`'s per-instance body is refactored to interleave hooks at
the two documented points (architecture-plan §3.3 step list). Only `def.hooks` entries whose `HookId`
is **both** present in the caster's `ActiveHooks` **and** registered in `HookRegistry` run — so an
un-acquired or not-yet-implemented hook is zero-cost:

```
1. params = resolve_params(...)
2. for (Pre, id)  in def.hooks where caster.ActiveHooks∋id && registry.get(id): hook.pre(&ctx, &mut params)
3. outcome = behavior.resolve(&ctx, &params)        // sees Pre-hook edits (e.g. doubled radius)
4. whiff gate / resolve_effects / apply_resolved_effects / VFX / projectile / zone   (unchanged)
5. for (Post, id) in def.hooks where caster.ActiveHooks∋id && registry.get(id): hook.post(&ctx, &outcome)
6. cooldown reset                                    (unchanged)
```

This is the §8.5 "split `execute_ready_abilities` into resolve/apply helpers around the hook points"
item. A hook `HookId` listed on an ability but absent from `HookRegistry` (e.g. `bone_shield_on_kill`
— its shield system is deferred) is **skipped silently** (an expected in-progress state during content
buildout, unlike an unregistered *behavior* which warns, because a missing behavior means the whole
ability is dead). `HookContext.zones` is fetched by execute as `Res<PlayerZonePresence>` (already in
the world after 2.1); no `&mut World` in the hook, consistent with `AbilityBehavior`.

**Neutrality of the split (critical — see §6).** In the golden campaign the only ability that lists a
hook is Death Strike (`Post "bone_shield_on_kill"`). Even if the bot acquires `bone_shield_epic` (it
is offerable, and any acquisition already sits in the current baseline), `bone_shield_on_kill` is
**not registered** ⇒ the Post loop finds nothing ⇒ no behavior. Blood Boil gets `hooks: [(Pre,
"blood_boil_dnd_range")]`, but that talent is held **out of the campaign-reachable pool** (§2.4), so
`ActiveHooks` never contains it ⇒ the Pre loop is empty. The refactor preserves the exact
resolve→behavior→effects→cooldown order for every existing cast ⇒ **byte-identical**.

### 2.4 The validation talent — Blood Boil range inside D&D

```rust
// ability/hooks.rs
struct BloodBoilDndRange;
impl AbilityHook for BloodBoilDndRange {
    fn pre(&self, ctx: &HookContext, params: &mut ResolvedParams) {
        if ctx.zones.is_inside("death_and_decay") {
            params.scale("radius", 2.0);   // Blood Boil's self_nova reads "radius"; Mechanics: "double range in D&D"
        }
    }
}
// AbilityPlugin::build: hooks.register("blood_boil_dnd_range", BloodBoilDndRange);
```

Wiring: `blood_boil.ability.ron` grows `hooks: [(Pre, "blood_boil_dnd_range")]`. The
`blood_boil_dnd_range_rare.talent.ron` stays `effect: Behavior("blood_boil_dnd_range")` (unchanged),
its comment corrected from "range" → "radius". **It is deliberately *not* added to
`blood_boil.talent_pool`** this phase, so the fixed-seed campaign cannot offer/acquire it (keeping the
master neutral — the Phase-5 "keep the spitter out of the campaign" precedent). It is validated
end-to-end by a scenario (§6.2). _Alternative on request:_ add it to the pool as real offerable
content and accept the resulting `talents`-column shift as a **declared** baseline regen.

This is architecture-plan §4's "Talent 3 — Zone-interaction" worked example, realized: no D&D code and
no base Blood Boil code is touched; the zone condition lives entirely in a ~5-line hook.

### 2.5 Zone occupant-tick effects (D2)

```rust
// zone/components.rs
#[derive(Component)]
pub struct ZoneEffects { pub damage_per_second: f32, pub regen_fraction: f32, pub tick: Timer }
// constants.rs: pub const ZONE_TICK_INTERVAL: f32 = 1.0;   // discrete 1 Hz ticks (deterministic, no per-frame float drift)
```

`zone_tick_effects` (in `CombatSet::Damage`, so its `DamageEvent`/`HealEvent` resolve the same frame,
like every other emitter):

- Advance each `ZoneEffects.tick`. On a completed tick, for that zone (position + radius + `Faction` +
  owner):
  - **Damage:** every actor of the **opposing** faction within radius → `DamageEvent { amount:
    damage_per_second, source: owner, tags: [Holy] }`. (Holy for Consecrated Ground; D&D's dps is 0 so
    it emits none.) Uses `Hurtbox`-free centre-distance (a ground AoE, matching `self_nova`).
  - **Regen:** if the **owner** is within radius and alive → `HealEvent { target: owner, amount:
    owner.max_health * regen_fraction }`.

Damage flows through the existing `apply_damage` (so `DamageTakenModifier` / `DamageDealtModifier` and
kill-credit all apply for free); regen through `apply_heal` (clamped to max). Consecrated Ground ships
as `consecrated_ground.ability.ron` (`activation: AutoCast`, `dropped_zone`, `damage_per_second`,
Holy) — an unbound demonstrator (no Paladin hero), exercised only by scenarios. **Neutral:** no zone
exists in the campaign ⇒ the system iterates nothing.

### 2.6 AMZ projectile-blocking (D2)

```rust
// zone/components.rs
#[derive(Component)] pub struct ZoneBlocksProjectiles;
```

`block_projectiles_in_zones` (in `CombatSet::Damage`, ordered **before** `projectile_collision` so a
blocked shot never lands): for each `ZoneBlocksProjectiles` zone and each projectile whose
`payload.target_faction == zone.Faction` (i.e. the shot is aimed at the side the zone protects) and
whose current position is inside the zone radius → **despawn the projectile**, *unless* the
projectile's `payload.source` entity is currently inside that same zone (Mechanics: "if enemies emit
projectiles from inside the zone it has no effect"). Uses only existing data (source entity +
`WorldPosition`); no new per-projectile state.

`amz.ability.ron`: DK band-4/6 ability (`amz` is already in the DK `band_4_6_pool`), `activation:
AutoCast`, `behavior: "dropped_zone"`, `zone: (zone_type: "amz", anchor: Fixed, blocks_projectiles:
true)`, `zone_radius`/`zone_duration`/`cooldown`. The epic "attached to you" variant (a talent that
flips `anchor` to `FollowCaster`) is deferred (§7); the `FollowCaster` mechanism itself is built (2.1)
and tested (§6.2 #8).

**Baseline note (the one measured risk — see §6/§8).** Making `amz` a live auto-cast requires adding
it to the `AbilityDef::MANIFEST` + creating its RON. The campaign draws exactly one band-4/6 ability
(deterministic by `GOLDEN_SEED`); today all three are inert (no RON/MANIFEST entry). If the seed draws
`amz`, it will now spawn AMZ zones in the campaign — a **genuine new behavior** ⇒ a *declared*
baseline regen. If it draws `abomination_limb`/`purgatory` (still no RON), the master stays
byte-identical. This is resolved empirically at step 6E, not guessed.

### 2.7 Frame timeline (unchanged skeleton)

No new system sets. New systems slot into the existing chain:

```
MovementSet::Intent      : player_input, flow-field rebuild, enemy AI, facing
MovementSet::Integrate   : apply_velocity → world_to_grid
                           → tick_zone_lifetimes → move_anchored_zones → build_player_zone_presence  ★ new
CombatSet::Damage        : tick_cooldowns → auto_cast → execute_ready_abilities (now runs Pre/Post hooks) ★
                           → block_projectiles_in_zones → move_projectiles → projectile_collision     ★ block before collide
                           → zone_tick_effects (damage opposing faction / regen owner)                 ★ new
CombatSet::Apply         : apply_damage, apply_heal
StatusSet::Tick/CrossInteract : …
CombatSet::Death         : enemy_death, player_death
```

---

## 3. File-level change map

| Area | File(s) | Change |
|---|---|---|
| Wire | `src/lib.rs` | `pub mod zone;` |
| Zone | `zone/plugin.rs` | replace `todo!()` — insert `PlayerZonePresence`; schedule lifetime/anchor/presence (Integrate) + `zone_tick_effects` + `block_projectiles_in_zones` (Damage), InRun-gated |
| Zone | `zone/components.rs` | add `Faction` to spawned zones (via bundle); `ZoneEffects`; `ZoneBlocksProjectiles` |
| Zone | `zone/systems/tick.rs` (new) | `zone_tick_effects` (occupant damage + owner regen) |
| Zone | `zone/systems/block.rs` (new) | `block_projectiles_in_zones` |
| Zone | `zone/systems/presence.rs`, `lifetime.rs` | keep (already written); minor: presence over `Fixed`/`Follow` centre already handled |
| Ability | `ability/assets.rs` | `AbilityDef.zone: Option<ZoneSpec>`; `ZoneSpec`; `ZoneAnchorKind`; parse tests |
| Ability | `ability/behavior.rs` | `DroppedZone` behavior; `CastOutcome.zone: Option<ZoneSpawn>`; `ZoneSpawn` |
| Ability | `ability/hooks.rs` (new) | `AbilityHook` trait, `HookContext`, `HookRegistry`, `BloodBoilDndRange` |
| Ability | `ability/behavior.rs` (ResolvedParams) | `set` / `scale` mutators |
| Ability | `ability/systems/execute.rs` | spawn zone from `def.zone`; **resolve/apply split** with Pre/Post hook loops; fetch `Res<PlayerZonePresence>` + `Res<HookRegistry>` + `ActiveHooks` |
| Ability | `ability/plugin.rs` | register `dropped_zone` behavior; build `HookRegistry` (register `blood_boil_dnd_range`) |
| Ability | `ability/mod.rs` | `pub mod hooks;` |
| Projectile | `projectile/systems/motion.rs` or `zone/systems/block.rs` | block ordered before `projectile_collision` |
| Core | `constants.rs` | `ZONE_TICK_INTERVAL` |
| Content | `assets/abilities/dnd.ability.ron` | add `zone` block; `damage_per_second → 0`; keep `Input` |
| Content | `assets/abilities/blood_boil.ability.ron` | `hooks: [(Pre, "blood_boil_dnd_range")]` |
| Content | `assets/abilities/{consecrated_ground,tree_conduit,amz}.ability.ron` (new) | demonstrators |
| Content | `assets/talents/blood_boil_dnd_range_rare.talent.ron` | comment fix (range→radius); effect unchanged |
| Manifest | `ability/assets.rs` `MANIFEST` | add `consecrated_ground`, `tree_conduit`, `amz` |
| Sim | `src/sim/mod.rs` | `zone_count`/`zones_of_type`/`player_in_zone`; `grant_talent`; await nothing new (zones need no lib) |
| Tests | `tests/zone.rs` (new); unit tests in the touched modules | scenarios + parse/behavior/hook units |
| Docs | this file §9; `CHANGELOG.md`; `architecture-plan.md` §8.5/§8.8/§8.1; `testing.md`; `Mechanics.md`; repo `CLAUDE.md` | 6F |

---

## 4. Content

### 4.1 Zone-emitting abilities (`*.ability.ron`)
| id | class/role | activation | behavior | zone_type | anchor | key params | occupant effect |
|---|---|---|---|---|---|---|---|
| dnd | BDK Special (L1) | Input | dropped_zone | death_and_decay | Fixed | zone_radius 80, zone_duration 8, cooldown 15, regen_percent_per_second 0.5 | owner regen only (dps 0) |
| tree_conduit | Druid (demo) | AutoCast | dropped_zone | tree_conduit | Fixed | zone_radius 70, zone_duration 6, cooldown 12 | none (marker; enhanced-attack consumer deferred) |
| consecrated_ground | Paladin (demo) | AutoCast | dropped_zone | consecrated_ground | Fixed | zone_radius 60, zone_duration 5, cooldown 3, damage_per_second 3 | Holy DoT to enemies inside |
| amz | BDK band-4/6 | AutoCast | dropped_zone | amz | Fixed | zone_radius 90, zone_duration 6, cooldown 12, blocks_projectiles | destroys enemy projectiles inside |

Numbers are tunable placeholders (working names — all `id`s stable). `tree_conduit`/`consecrated_ground`
are unbound demonstrators (no Druid/Paladin hero), exercised by scenarios only.

### 4.2 Talent
| id | ability_scope | rarity | uniqueness | effect |
|---|---|---|---|---|
| blood_boil_dnd_range_rare (existing) | blood_boil | Rare | Exclusive | `Behavior("blood_boil_dnd_range")` — Pre hook, ×2 `radius` inside `death_and_decay` |

Registered behaviors after Phase 6: `melee_cone`, `self_nova`, `projectile`, `contact_melee`,
**`dropped_zone`**. Registered hooks: **`blood_boil_dnd_range`** (`bone_shield_on_kill` still
unregistered → inert; `summon`/`orbiting`/`leap_to_target`/`channel_while_moving` still unregistered).

---

## 5. Implementation sequence (each step is independently `/compat-check`-able)

Ordered so behavior stays **unchanged** until the single measured step 6E.

**6A — Zone core (behavior-neutral).** ★ Gate: **baseline byte-identical.**
`mod zone`; `ZonePlugin` (presence + lifetime + follow-anchor) wired into `GameLogicPlugin`;
maintenance scheduled at the end of `MovementSet::Integrate`. No ability spawns a zone yet ⇒ every
zone system is an empty loop.

**6B — `dropped_zone` + `ZoneSpec` + D&D/Tree Conduit live (neutral).** ★ Gate: **byte-identical.**
`AbilityDef.zone`; `DroppedZone` + `CastOutcome.zone`; execute spawns `PersistentZone` (+ `Faction`);
register `dropped_zone`; wire `dnd`/`tree_conduit`. D&D stays `Input`, campaign never fires it ⇒ no
zone spawns in the master.

**6C — Hook system + execute split + validation talent (neutral).** ★ Gate: **byte-identical.**
`ability/hooks.rs`; execute resolve/apply split with Pre/Post loops; `ResolvedParams::set/scale`;
register `blood_boil_dnd_range`; `blood_boil.ability.ron` `hooks`. Talent held out of the campaign
pool. **Verify the split neutrality argument (§2.3):** no registered hook is active on any
campaign-cast ability ⇒ no diff.

**6D — Zone occupant-tick effects (neutral).** ★ Gate: **byte-identical.**
`ZoneEffects` + `zone_tick_effects` (Damage set); `consecrated_ground` demonstrator; D&D regen live.
No zone in the campaign ⇒ neutral.

**6E — AMZ projectile-blocking (neutral OR declared — the one measured step).** ★
`ZoneBlocksProjectiles` + `block_projectiles_in_zones` (before `projectile_collision`);
`amz.ability.ron` + `MANIFEST`. **Run the golden master and read the result:** if the fixed seed does
**not** draw `amz` → byte-identical (commit as-is); if it **does** → the AMZ zone spawns are a genuine
new behavior → verify the diff is a coherent AMZ effect (zones appear near the player; enemy bolts
that used to hit now vanish; no *unrelated* column moves), then `UPDATE_GOLDEN=1` with a CHANGELOG
entry. Either way is legitimate; a regression is any diff that is *not* explained by AMZ.

**6F — Docs + final gate.** §9 as-built; CHANGELOG "Phase 6"; architecture-plan §8.5 (retire the zone
scaffold + `execute_ready_abilities` split rows; note the hook registry is now built),
§8.8 "Phase 6 delivered", §8.1(8) (AMZ blocking done); testing.md Phase-6 line already stubbed → mark
delivered; Mechanics.md working-name notes (Tree Conduit/Consecrated Ground/AMZ are demonstrators);
repo `CLAUDE.md` debt update. Full `/compat-check`; classify the 6E result.

---

## 6. Validation & testing suite

### 6.1 Unit tests (`src/**` `#[cfg(test)]`)
- `ability/assets.rs` — parse `dnd`/`tree_conduit`/`consecrated_ground`/`amz` (`zone` block: type,
  anchor, `blocks_projectiles`; dps/regen params).
- `ability/behavior.rs` — `DroppedZone` returns a `ZoneSpawn` at origin, `needs_aim() == false`.
- `ability/hooks.rs` — `BloodBoilDndRange.pre` doubles `radius` iff a fake `PlayerZonePresence`
  contains `death_and_decay`, leaves it otherwise; `ResolvedParams::scale`/`set` math.
- Hook gating (pure) — a `def.hooks` entry runs only when `ActiveHooks ∋ id` **and** the registry has
  it (matrix: acquired+registered → runs; acquired+unregistered → skip; unacquired → skip).

### 6.2 Golden scenarios (`tests/zone.rs`)
1. **dnd_cast_spawns_zone_and_expires** — `trigger_ability("dnd")` ⇒ one `PersistentZone`
   (`death_and_decay`) at the player; present in `PlayerZonePresence`; gone after `zone_duration`.
2. **presence_tracks_enter_and_exit** — teleport the player out of the D&D radius ⇒ presence clears;
   back in ⇒ set. (Locks the spatial cache.)
3. **dnd_doubles_blood_boil_range_inside** — *(testing.md Phase-6 DoD)* grant `blood_boil` +
   `blood_boil_dnd_range_rare` (new `Sim::grant_talent`); place an enemy between `radius` and
   `2·radius`. Outside D&D: Blood Boil auto-cast does **not** hit it. Standing in D&D: it **does**
   (the Pre hook doubled `radius`). The zone condition is the only difference.
4. **zone_dot_damages_opposing_faction_only** — a Consecrated Ground zone: an enemy inside loses
   `damage_per_second` per tick; an enemy outside is untouched; the Friendly player inside is unharmed
   (faction gate).
5. **dnd_regen_heals_owner_inside** — player at reduced HP inside D&D heals per tick; steps outside ⇒
   healing stops. (No enemy damage from D&D — dps 0.)
6. **amz_blocks_enemy_projectile** — a spitter bolt aimed at the player is destroyed upon entering an
   AMZ zone (never reaches the player); a control run without AMZ lands the hit.
7. **amz_ignores_projectile_emitted_from_inside** — a bolt whose `source` stands inside the AMZ passes
   (the Mechanics exception).
8. **follow_anchor_zone_tracks_owner** — a `FollowCaster` zone's centre (and the player's presence)
   moves with the player across several frames (the AMZ-epic mechanism).

### 6.3 Golden master
Neutral through 6A–6D. At **6E** it is neutral iff the fixed seed does not draw `amz`; if it does, a
**declared** regen for the AMZ mechanic (contact/values of every other column must be unchanged —
verify before regenerating). The reproducibility tripwire must still pass: zones, hooks, and ticks
carry **no RNG** (zone-tick cadence is a fixed 1 Hz timer; hook edits are deterministic).

### 6.4 Compat gate
`/compat-check` at every ★. 6A–6D expect **no** diff; 6E expects **either** no diff **or** exactly the
AMZ-attributable diff. Any diff not explained by AMZ, or any drift in 6A–6D, is a regression — bisect
with the §6.2 scenarios.

---

## 7. Deferred — with the trigger that revives each

| Deferred | Revived by |
|---|---|
| Death Strike bonus damage inside D&D; Heart Strike +1 target inside D&D | Heart Strike ability + a Death-Strike zone hook (Phase 9 BDK content) |
| Tree Conduit "enhanced next animal attack" consumer | Druid enhanced-attack state machine (Phase 9) |
| AMZ epic "attached to you" talent (flip anchor → FollowCaster) | BDK talent content (Phase 9); the anchor mechanism ships now |
| Bone-shield Post hook implementation | the shield/absorb system (§8.1(5)) — its `ActiveHook`/`hooks` plumbing rides along inert |
| Consecrated Ground as a real Paladin ability + its talents | Paladin hero (Phase 9); ships as a demonstrator |
| Zone visuals (translucent disc on `Added<PersistentZone>`) | a presentation pass (presentation-only; would not move the baseline) |
| Projectiles ignore walls | owner-accepted (2026-07-05); revisit at Mage playtest |

---

## 8. Risks & mitigations

| Risk | Mitigation / expected outcome |
|---|---|
| The `execute_ready_abilities` resolve/apply split shifts the master | Preserve the exact resolve→behavior→effects→cooldown order; hooks run only when acquired **and** registered; §2.3 shows no such hook is active on a campaign cast ⇒ 6C byte-identical. |
| Zone maintenance reorders the movement tie-break (px/py drift) | Placed at the end of `MovementSet::Integrate` (positions pinned since Phase 3.1); zone systems never write an actor `WorldPosition`. |
| 6E moves the master unexpectedly | It is the *only* step that can. Measure it in isolation; a diff explained wholly by AMZ zones is a declared regen, anything else is a regression (bisect with #6/#7). |
| AMZ blocking too aggressive (kills player shots, or shots from inside) | Gate on `payload.target_faction == zone.Faction` (only shots aimed at the protected side) **and** source-not-inside (the exception). Scenarios #6/#7 lock both directions. |
| Zone DoT float drift / nondeterminism | Fixed 1 Hz tick emitting whole `damage_per_second`; no RNG; reproducibility tripwire covers it. |
| Hook borrow tangle (mutating params mid-execute) | `HookContext` carries only `PlayerZonePresence`; execute already owns the resources; hook is `&mut World`-free like `AbilityBehavior`. |
| Regen interpretation (`regen_percent_per_second`) | Treated as a fraction of `max_health` per tick (0.5 → 0.5%/s); documented here + asserted in #5; tunable in RON. |

---

## 9. As-built notes (completed 2026-07-05)

Phase 6 landed as planned across the six steps, at **full scope** (D2) and with the **code-driven
hook system** (D1). Like Phase 5, **the golden master moved zero times** — byte-identical at every
step, including the "measured" 6E.

- **The whole phase was baseline-neutral — no regeneration.** 6A–6D were neutral by construction (no
  zone exists in the campaign: D&D is `Input` and the bot never fires it; the validation talent is
  held out of the offerable pool; occupant ticks iterate nothing). **6E was neutral too**, more
  strongly than budgeted: adding `amz` to the manifest makes it a live auto-cast *if* the fixed seed
  unlocks it, but AMZ zones appear in **no snapshot column** (the trace records hp/level/xp/enemies/
  abilities/talents/statuses/pos — never zones), and the campaign has **no enemy projectiles** for
  AMZ to block. So the "declared regen if the seed draws amz" contingency never triggered.
- **The code-driven hook system shipped (D1) and pays the §8.5 "execute split" debt.** `ability/hooks.rs`
  = `AbilityHook` trait (`pre`/`post`, both defaulting to no-ops) + `HookContext` (caster + zone
  presence) + `HookRegistry` (mirrors `BehaviorRegistry`). `execute_ready_abilities` now interleaves
  hooks at the resolve→behavior boundary (Pre, may mutate `ResolvedParams`) and after apply (Post,
  reads the outcome), each gated on the caster's `ActiveHooks` **and** registration. The only
  registered hook is `blood_boil_dnd_range` (Pre, ×2 `radius` inside D&D). `bone_shield_on_kill`
  stays **unregistered → inert** (its shield system is deferred, §8.1(5)); its plumbing (ActiveHook +
  the `hooks` list on death_strike) rides along for free. **Split verified byte-identical:** no
  registered hook is active on any campaign-cast ability.
- **Schema (as designed).** `AbilityDef.zone: Option<ZoneSpec>` (`zone_type` + `anchor` + `blocks_projectiles`);
  the `dropped_zone` behavior returns a `CastOutcome.zone` request and execute builds the
  `PersistentZone` from the spec + resolved params + the caster's `Faction` — exactly the
  `projectile` pattern. RON `Option` fields use `Some((...))` (this repo doesn't enable RON
  `implicit_some`).
- **Latent guard closed.** Zones carry `WorldPosition` + `Faction` (since 6B), so the execute
  candidate-gather query would otherwise pick them up as targets. Added `Without<PersistentZone>` to
  that query (and to the `zone_tick_effects` occupant query). Neutral (no zones in the campaign), but
  correct — a friendly zone can never be an enemy cast's target/primary.
- **D&D is a buff zone.** Per Mechanics, D&D deals no enemy damage — its `damage_per_second` was set
  to **0** (the `2.0` scaffold value dropped) and only `regen_percent_per_second` (owner heal, 0.5%
  of max/tick) is live. The cross-ability buffs (Death Strike damage / Heart Strike targets inside)
  stay deferred to the BDK content pass (they need Heart Strike + a Death-Strike zone hook).
- **Occupant ticks (6D).** `ZoneEffects { damage_per_second, regen_fraction, tick }` on a 1 Hz
  repeating timer; `zone_tick_effects` (CombatSet::Damage) emits a Holy DoT to opposing-faction
  occupants (Consecrated Ground demonstrator) and regen to the owner inside (D&D). Deterministic —
  discrete ticks, no RNG.
- **AMZ blocking (6E).** `ZoneBlocksProjectiles` marker + `block_projectiles_in_zones`, ordered
  `after(move_projectiles).before(projectile_collision)` so a blocked shot never lands. Blocks a
  projectile whose `target_faction == zone.Faction` while inside the zone, **unless its source
  stands inside** (the "emitted from inside" exception). The `FollowCaster` anchor mechanism is built
  + tested; the talent that flips base AMZ to follow is deferred content.
- **Demonstrators.** `tree_conduit` (marker-only, its enhanced-attack consumer deferred),
  `consecrated_ground` (Holy DoT), `amz` (blocking) join the manifest. Tree Conduit / Consecrated
  Ground are unbound (no Druid/Paladin hero), like the Phase-3 Fireblast/Frostbolt/Scratch.
- **Tests: 107 passing** (was 94): +5 unit (zone RON parse ×4: tree_conduit/consecrated/amz/non-zone;
  the `blood_boil_dnd_range` hook) and +8 golden scenarios (`tests/zone.rs`). Build warning-free.
  **Golden baseline unchanged (no regeneration).** New sim helpers: `zone_count`/`zone_types`/
  `zone_center`/`player_in_zone`/`spawn_zone`/`grant_talent`.
- **Debt (architecture-plan §8.5/§8.8):** the `execute_ready_abilities` **resolve/apply split is
  done** (first code-driven hook landed); **AMZ blocking done** (§8.1(8) fully closed). Still open:
  the bone-shield Post hook impl (needs the shield/absorb system, §8.1(5)); cross-ability zone buffs
  + Tree Conduit's enhanced-attack consumer + the AMZ-follow talent (Phase 9 class content); zone
  visuals (presentation pass). Wall-collision remains owner-accepted.
