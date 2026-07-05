# Phase 3 Implementation Plan — Status Effects (+ Auto-cast, Generic Effects, Projectiles)

_Written 2026-07-05 against `main` @ `884a406` (Phases 0–2 + testing infra complete).
Companion to `docs/architecture-plan.md` (§3.5 status, §8 amendments) and `docs/testing.md`._

---

## 0. Decisions locked for this phase

Three consequential decisions were resolved with the project owner before planning; they widen
Phase 3 beyond the two bullets in architecture-plan §7 and set its schema direction.

| # | Decision | Consequence |
|---|---|---|
| **D1** | **Scope = status effects + CC + fold in auto-cast (§8.1.1 / "Phase 3.5").** The *general* actor stat-sheet (crit/attack-speed/generic move-speed — §8.1.4 / "3.75") stays deferred. | Passive DoT-appliers can fire on cooldown without input. Frostbite's *own* slow & damage-taken land here (they are the effect); a generic stat sheet does not. |
| **D2** | **Ability→effect declaration = full generic effect list.** RON gains `effects: Vec<EffectSpec>` unifying damage / heal / leech / apply-status as declarative data. | Behaviors stop hardcoding damage/leech; they resolve *hits*, the engine applies the *effects*. A new status-applying ability is data-only. Requires refactoring the Phase-1 `AbilityEffect`/`MeleeCone` model (compat-critical). |
| **D3** | **Demonstrators = pull Mage projectiles forward.** Register the `projectile` behavior (overdue Phase-1 debt) and add **Fireblast** (→ blaze) and **Frostbolt** (→ frostbite) as standalone ability RONs, plus a bleed **Scratch** cone. Not yet class/stance-bound (that stays Phase 4). | Cross-element cancellation (fire↔frost) is tested faithfully with real content. Projectile motion/collision is built now. Class wiring stays out. |

Nothing below should require touching class/stance code, the act graph, persistence, or the UI
beyond what already exists.

---

## 1. Scope

### In scope
1. **Generic effect model** — `effects: Vec<EffectSpec>` on `AbilityDef`; behaviors return a
   `CastOutcome` (hits + optional projectile + optional VFX shape); one shared applier drives
   both instant and projectile-delivered effects. (D2)
2. **Status core** — `StatusEffectDef` asset + `.status.ron` loader; per-target status *instance*
   entities; apply / tick / remove lifecycle honoring `StackingRule`; `ApplyStatusEvent` /
   `RemoveStatusEvent`; `StatusSet` wired into the combat chain.
3. **Cross-element cancellation** — `DamageEvent.tags` (Phase-0 field, first consumer) →
   `removed_by_tags` → removal. Fire removes frostbite; frost removes blaze; blaze's Fire tick
   clears frostbite emergently.
4. **CC & status-stat integration** — root/stun immobilize; frostbite slows (×0.8 move) and
   raises damage taken (×1.1); resolved per-actor and fed back into movement (`apply_velocity`)
   and damage (`apply_damage`) via generic modifier components (no core→status coupling).
5. **DoT kill credit** — a DoT tick carries its applier as `source`; a lethal tick credits XP.
6. **Projectile behavior** — travelling projectile with movement, radius collision, pierce,
   lifetime; carries its resolved effect list to impact. (D3)
7. **Auto-cast** — `Activation::AutoCast` fires an ability on cooldown with no input. Vehicle:
   **Blood Boil** (BDK L2/3, already unlocked inert) goes live as a periodic self-nova. (D1)
8. **Demonstrator content** — Fireblast, Frostbolt, Scratch, Blood Boil RONs + `self_nova`
   behavior; rewrite the six status RON files to the declarative schema.
9. **Full test suite** — unit tests, one golden scenario per mechanic, golden-master baseline
   regenerated (declared change), CHANGELOG + docs.

### Out of scope (explicitly deferred — see §7)
General actor stat sheet (crit/attack-speed/generic move-speed); shields/absorbs (bone shield,
ice barrier); forced movement (knockback, Abomination-limb grip); status-consuming talent hooks
(Ferocious Bite consumes bleed, etc.); enemy abilities/projectiles (Phase 5); persistent zones
(Phase 6); class & stance binding of the new abilities (Phase 4); the `StatusHookRegistry`
(introduced lazily when the first *code-driven* status hook actually lands — the six built-ins
are fully declarative and need zero hooks).

---

## 2. Architecture

### 2.1 Generic effect model (D2)

Today `MeleeCone::execute` computes cone membership *and* hardcodes damage + leech + VFX, pushing
`AbilityEffect::{Damage,Heal,ConeVfx}`. Phase 3 splits **targeting** (behavior) from **outcome**
(data):

```rust
// ability/behavior.rs
pub struct HitTarget { pub entity: Entity, pub pos: Vec2 }

/// What a behavior resolves for one cast. The execute system applies the ability's
/// declarative `effects` against this, and (if present) spawns the projectile.
pub struct CastOutcome {
    pub origin: Vec2,
    /// Immediate hits (cone / nova / single-target melee). Empty for pure projectile casts.
    pub hits: Vec<HitTarget>,
    /// The "primary" hit (nearest / first) for PrimaryHit-scoped effects.
    pub primary: Option<HitTarget>,
    /// Deferred delivery: execute spawns a projectile carrying the resolved effects.
    pub projectile: Option<ProjectileSpawn>,
    /// Shape VFX for the presentation layer (unchanged cone flash, etc.).
    pub vfx: Option<VfxShape>,
}

pub trait AbilityBehavior: Send + Sync + 'static {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome;
    /// Direction-dependent shapes (cone, projectile) return true; self-centred (nova) false.
    fn needs_aim(&self) -> bool { true }
}
```

`AbilityDef` gains a declarative effect list (float-only `base_params` stays; effects reference
param *keys*, so the talent modifier stack still reaches every number):

```rust
#[serde(default)] pub effects: Vec<EffectSpec>,
#[serde(default)] pub activation: Activation,   // Input (default) | AutoCast   (D1)

pub enum EffectSpec {
    Damage      { amount: StatId, tags: Vec<DamageTag>, target: EffectTarget },
    Heal        { amount: StatId, target: EffectTarget },
    Leech       { percent: StatId },                          // heals caster % of damage dealt this cast
    ApplyStatus { status: StatusEffectId, stacks: u8, target: EffectTarget },
}
pub enum EffectTarget { AllHits, PrimaryHit, Caster }
```

`#[serde(default)]` keeps every existing RON parsing (an un-migrated ability = empty effects =
inert). The execute system replaces its `apply_effects` with a shared applier:

```rust
// resolves EffectSpec + CastOutcome → DamageEvent / HealEvent / ApplyStatusEvent / VFX.
// Used by BOTH execute_ready_abilities (instant) and projectile_collision (deferred).
fn apply_effects(w: &mut EffectWriters, source: Entity, outcome_hits: &Hits,
                 origin: Vec2, effects: &[ResolvedEffect]);
```

**Migration of Death Strike** (must be numerically identical — see §5.3A):
```ron
behavior: "melee_cone",
effects: [
    Damage(amount: "damage", tags: [Physical], target: AllHits),
    Leech(percent: "leech_percent"),
],
// range / half_angle drive the cone shape + VFX inside the melee_cone behavior.
```

### 2.2 Status effect core

**Schema evolution (declarative, not hook-first).** The scaffold sketched behavior as opaque
`on_apply_hooks / on_tick_hooks / on_remove_hooks: Vec<HookId>`. Phase 3 makes the six built-ins
fully declarative — zero Rust per effect — and keeps a `hooks` escape hatch (empty for now) for
future code-driven effects:

```rust
// status/assets.rs  — loaded from assets/status_effects/<id>.status.ron
pub struct StatusEffectDef {
    pub id: StatusEffectId,
    pub display_name: String,
    pub stacking: StackingRule,               // RefreshOnReapply | StackCapped(u8) | StackUnlimited
    pub base_duration_secs: f32,
    pub tick: Option<TickSpec>,               // DoT: interval + flat damage + tags
    pub move_speed_mult: f32,                 // 1.0 = none; frostbite 0.8
    pub damage_taken_mult: f32,               // 1.0 = none; frostbite 1.1
    pub immobilize: bool,                     // root, stun  → velocity zeroed
    pub suppress_abilities: bool,             // stun        → (enemy casts; Phase 5) / player
    pub removed_by_tags: Vec<DamageTag>,      // fire clears frostbite, frost clears blaze
    pub removes_on_apply: Vec<StatusEffectId>,
    #[serde(default)] pub hooks: Vec<HookId>, // escape hatch; empty for the six built-ins
}
pub struct TickSpec { pub interval_secs: f32, pub damage: f32, pub tags: Vec<DamageTag> }
```

**Instances.** One child entity per active instance, mirroring §3.5:

```rust
pub struct StatusEffectInstance {
    pub def_id: StatusEffectId,
    pub target: Entity,      // stored directly (like AbilityInstance.owner) → no relationship join in queries
    pub source: Entity,      // applier → DoT kill credit / attribution
    pub timer: Timer,        // duration (Once)
    pub tick_timer: Option<Timer>,  // Repeating, only if def.tick.is_some()
}
```

Instances are **also parented to the target via `ChildOf`** (Bevy 0.16 relationship — note the
scaffold's stale `&Parent`) so Bevy's recursive `despawn()` in `enemy_death` cleans them up for
free. The `target` field is the query key; `ChildOf` is purely for lifecycle. A defensive
`despawn_orphaned_status` sweep covers any non-recursive despawn path.

**Lifecycle systems** (`status/systems/`):
- `apply_status_effects` — drains `ApplyStatusEvent`; per `StackingRule`: `RefreshOnReapply`
  resets the existing instance's timer (≤1 instance); `StackCapped(n)` spawns only while count
  < n; `StackUnlimited` always spawns. Also applies `removes_on_apply`.
- `tick_status_effects` — advances `timer` + `tick_timer`; on a tick boundary emits
  `DamageEvent { source, tags: def.tick.tags, .. }`; despawns expired instances.
- `apply_cross_interactions` — for each `DamageEvent` this frame, removes target instances whose
  `removed_by_tags` intersect the event tags (emits `RemoveStatusEvent`).
- `remove_status_effects` — drains `RemoveStatusEvent`, despawns matching instances.
- `resolve_actor_status` — recomputes the per-actor modifier components (§2.3) from the settled
  instance set; runs last so removals are reflected.

### 2.3 CC & status-stat integration (no core→status coupling)

Core stays ignorant of *which* status caused an effect; it reads generic modifier components that
the status system owns:

```rust
// core/components.rs  (new, generic — not status-specific)
#[derive(Component)] pub struct MoveSpeedModifier(pub f32);   // default 1.0
#[derive(Component)] pub struct DamageTakenModifier(pub f32); // default 1.0
#[derive(Component)] pub struct Immobilized;                  // marker; present ⇒ velocity zeroed
```

`resolve_actor_status` folds the active instances on each actor into these (product of
`move_speed_mult`, product of `damage_taken_mult`, `Immobilized` iff any `immobilize`).

Integration points (both minimal, generic):
- **Movement** — new `apply_movement_status` system: `vel *= MoveSpeedModifier; if Immobilized { vel = 0 }`,
  ordered `.after(enemy_follow_flow_field).after(player_input).before(apply_velocity)`.
- **Damage** — `apply_damage` multiplies `event.amount` by the target's `DamageTakenModifier`
  (default 1.0 if absent). This is the "read by apply_damage.rs in Phase 3" the frostbite.ron
  comment predicted.

`suppress_abilities` (stun) has no live consumer in Phase 3 (enemies gain abilities in Phase 5;
nothing stuns the player yet) — resolved into a component and left for Phase 5 to read.

### 2.4 Projectile behavior (D3)

The provisional `projectile` module (currently VFX-only) grows real motion + collision:

```rust
// projectile/components.rs
#[derive(Component)] pub struct ProjectileMotion { pub velocity: Vec2, pub radius: f32, pub pierce_remaining: u32 }
#[derive(Component)] pub struct ProjectilePayload { pub source: Entity, pub effects: Vec<ResolvedEffect>, pub already_hit: Vec<Entity> }
```

- The `projectile` behavior's `resolve` returns `CastOutcome { hits: [], projectile: Some(spawn), .. }`.
- `execute_ready_abilities` spawns the projectile entity carrying the ability's **resolved**
  effects (params already numeric) + motion, then applies nothing instantly.
- `move_projectiles` (in `CombatSet::Damage`, before collision) integrates position.
- `projectile_collision` (in `CombatSet::Damage`) tests each projectile against enemies within
  `radius`, and on first contact runs the carried effects through the **shared** `apply_effects`
  (§2.1) — so Fireblast's `Damage(Fire)` + `ApplyStatus(blaze)` fire on impact, not at cast.
  Decrements `pierce_remaining`; despawns at 0 or on `Lifetime` expiry.

This keeps status-on-projectile fully data-driven (the effect list travels with the projectile).

### 2.5 Auto-cast (D1)

- `Activation::AutoCast` abilities are fired by a new `auto_cast_abilities` system: for each
  `AbilityInstance` whose def is `AutoCast` and whose cooldown is ready, emit a
  `TriggerAbilityEvent` (reusing the entire existing execute path). Ordered before
  `execute_ready_abilities` in `CombatSet::Damage`.
- The blanket "skip if `Facing == 0`" gate moves out of `execute_ready_abilities` and becomes
  per-behavior (`needs_aim()`), so a self-centred nova auto-casts without aim while a cone/
  projectile still waits for aim (and does **not** consume its cooldown when aimless).
- **Vehicle: Blood Boil** — already unlocked as an inert BDK L2/3 instance. New `self_nova`
  behavior (all enemies within `radius` of the caster), `activation: AutoCast`,
  `effects: [Damage(Physical, AllHits), Leech]`. This makes Blood Boil live and is the one
  change that intentionally moves the golden-master baseline (§6).

### 2.6 Frame timeline (deterministic ordering)

All within `Update`; single-threaded in the sim. New/changed steps in **bold**.

```
1  player_input, enemy_follow_flow_field                    (velocity setters)
2  apply_movement_status         ← slow × mult, immobilize × 0     [.after setters .before apply_velocity]  ★
3  apply_velocity → world_to_grid
   ── CombatSet::Damage ──
4  auto_cast_abilities → execute_ready_abilities             (emit DamageEvent, ApplyStatusEvent, spawn projectiles)  ★
5  move_projectiles → projectile_collision                   (emit DamageEvent/ApplyStatusEvent on impact)  ★
6  enemy_attack
   ── CombatSet::Apply ──
7  apply_damage  (× DamageTakenModifier)  ,  apply_heal       ★ (damage now scaled)
   ── StatusSet::Tick ──
8  apply_status_effects → tick_status_effects                (spawn/refresh; DoT emits DamageEvent for next frame)  ★
   ── StatusSet::CrossInteract ──
9  apply_cross_interactions → remove_status_effects → resolve_actor_status   ★
   ── CombatSet::Death ──
10 enemy_death, player_death
```

**Consequences (all deterministic, pinned by scenarios):**
- Status *stat* effects (slow, immobilize, damage-taken) lag application/removal by **one frame**
  (resolved at step 9, consumed at steps 2/7 next frame). ~16 ms; documented.
- DoT damage lags one frame (emitted step 8, applied step 7 next frame).
- A Fire hit that *removes* frostbite still lands with frostbite's ×1.1 that frame (removal is
  step 9, after that frame's apply_damage) — "the hit that breaks the ice still benefits."

`CorePlugin` extends the set chain to
`(CombatSet::Damage, CombatSet::Apply, StatusSet::Tick, StatusSet::CrossInteract, CombatSet::Death).chain()`.

---

## 3. File-level change map

| Area | File(s) | Change |
|---|---|---|
| Effect model | `ability/behavior.rs` | `CastOutcome`/`HitTarget`; `AbilityBehavior::resolve` + `needs_aim`; rewrite `MeleeCone`; add `SelfNova`, `Projectile` behaviors |
| Effect model | `ability/assets.rs` | `effects: Vec<EffectSpec>`, `activation: Activation`, `EffectSpec`, `EffectTarget` (all `#[serde(default)]`) |
| Effect model | `ability/systems/execute.rs` | behavior→outcome; shared `apply_effects`; `auto_cast_abilities`; per-behavior aim gate |
| Effect model | `ability/plugin.rs` | register `self_nova`, `projectile`; add `auto_cast_abilities` to the chain |
| Status | `status/assets.rs` | declarative `StatusEffectDef` + `TickSpec`; `StatusEffectDefLoader` (`status.ron`); `StatusLibrary` |
| Status | `status/components.rs` | `StatusEffectInstance{target,source,tick_timer}`; keep events; drop `&Parent` usage |
| Status | `status/systems/{apply,tick,cross_interact,remove,resolve}.rs` | implement lifecycle + `resolve_actor_status` |
| Status | `status/plugin.rs` | replace `todo!()`: assets, events, `StatusLibrary` load, systems into `StatusSet` |
| Core | `core/components.rs` | `MoveSpeedModifier`, `DamageTakenModifier`, `Immobilized` |
| Core | `core/systems/apply_damage.rs` | multiply by `DamageTakenModifier` |
| Core | `core/systems/movement.rs` or new | `apply_movement_status` |
| Core | `core/plugin.rs` | extend the set chain; register `apply_movement_status` ordering |
| Projectile | `projectile/components.rs`, `systems/{move,collision}.rs`, `plugin.rs` | real motion + collision + payload |
| Game wiring | `game/plugin.rs` | add `StatusPlugin` to `GameLogicPlugin` |
| Content | `assets/status_effects/*.status.ron` (×6, renamed) | declarative rewrite |
| Content | `assets/abilities/{fireblast,frostbolt,scratch,blood_boil}.ability.ron` (new) | demonstrators + effects/activation |
| Content | `assets/abilities/death_strike.ability.ron` | migrate to `effects` list |
| Sim | `src/sim/mod.rs` | helpers: `status_instances(entity)`, `has_status`, `spawn_dummy` (high-HP static target), load new abilities |

The scaffold comments in `status/*` that reference `&Parent`, `HookRegistry`, and hook-first
semantics are corrected to match this declarative model as each file is implemented.

---

## 4. Content

### 4.1 Status defs (declarative rewrite, `*.status.ron`)

| id | stacking | dur | tick | move× | dmg-taken× | immob | suppress | removed_by | notes |
|---|---|---|---|---|---|---|---|---|---|
| bleed | RefreshOnReapply | 4.0 | 1.0s / 3 dmg / [Physical] | 1.0 | 1.0 | – | – | – | Druid DoT |
| blaze | RefreshOnReapply | 4.0 | 1.0s / 3 dmg / [Fire] | 1.0 | 1.0 | – | – | [Frost] | Fire tick clears frostbite emergently |
| frostbite | RefreshOnReapply | 5.0 | – | 0.8 | 1.1 | – | – | [Fire] | slow + amp |
| holy_mark | RefreshOnReapply | 6.0 | – | 1.0 | 1.0 | – | – | – | consumed by Paladin (Phase 9) |
| root | RefreshOnReapply | 2.5 | – | 1.0 | 1.0 | ✓ | – | – | immobilize |
| stun | RefreshOnReapply | 1.5 | – | 1.0 | 1.0 | ✓ | ✓ | – | immobilize + cast-lock |

Talent-driven variants (Mega Bleed → `StackCapped(3)`, frostbite ×3) stay deferred to Phase 4;
`StackCapped` is implemented and unit-tested with a synthetic def so it is ready.

### 4.2 New demonstrator abilities (unbound; triggerable in tests & optionally hand-bound)

```ron
// fireblast.ability.ron   (Mage fire basic — projectile)
behavior: "projectile", activation: Input,
base_params: { "damage": 8.0, "speed": 320.0, "radius": 8.0, "range": 400.0, "pierce": 0.0, "cooldown": 0.8 },
effects: [ Damage(amount:"damage", tags:[Fire],  target:PrimaryHit),
           ApplyStatus(status:"blaze",     stacks:1, target:PrimaryHit) ],

// frostbolt.ability.ron   (Mage frost basic — projectile)
effects: [ Damage(amount:"damage", tags:[Frost], target:PrimaryHit),
           ApplyStatus(status:"frostbite", stacks:1, target:PrimaryHit) ],

// scratch.ability.ron     (Druid animal basic — cone applying bleed)
behavior: "melee_cone",
effects: [ Damage(amount:"damage", tags:[Physical], target:AllHits),
           ApplyStatus(status:"bleed", stacks:1, target:AllHits) ],

// blood_boil.ability.ron  (BDK L2/3 — auto-cast self nova, goes LIVE)
behavior: "self_nova", activation: AutoCast,
base_params: { "damage": 6.0, "radius": 90.0, "leech_percent": 5.0, "cooldown": 4.0 },
effects: [ Damage(amount:"damage", tags:[Physical], target:AllHits), Leech(percent:"leech_percent") ],
```

Registered behaviors after Phase 3: `melee_cone`, `self_nova`, `projectile`
(`dropped_zone`/`orbiting`/`summon`/… stay unregistered → inert, as today).

---

## 5. Implementation sequence (each sub-step is independently `/compat-check`-able)

Ordered so the build stays green and behavior stays *unchanged* until the deliberate baseline
move in 3E. Run `/compat-check` at every ★ boundary.

**3A — Generic effect refactor (behavior unchanged).** ★
Introduce `CastOutcome`/`EffectSpec`; rewrite `MeleeCone` to resolve hits; extract shared
`apply_effects`; migrate `death_strike.ability.ron` to the `effects` list. Rewrite the
`ability/behavior.rs` unit tests to assert on `CastOutcome` + effect resolution.
**Gate: `tests/combat.rs` and the golden-master baseline must be byte-identical** — this is a
pure refactor. Any movement is a regression to fix, unless float reassociation is unavoidable
(then declare it).

**3B — Status core (no applier yet).** ★
`StatusEffectDef` + loader + `StatusLibrary`; rewrite the six RON files; instance entities;
`apply/tick/remove` systems; `StatusSet` into the chain; `StatusPlugin` into `GameLogicPlugin`;
`EffectSpec::ApplyStatus` handled in `apply_effects`. No shipped ability applies a status yet →
baseline still unchanged. Unit tests: RON round-trips, stacking, cross-tag matching.

**3C — CC & stat integration.** ★
`MoveSpeedModifier`/`DamageTakenModifier`/`Immobilized`; `resolve_actor_status`;
`apply_movement_status`; `apply_damage` scaling; `apply_cross_interactions`. Still no shipped
applier → baseline unchanged. Scenarios drive it via `ApplyStatusEvent` + direct `DamageEvent`.

**3D — Projectile behavior.** ★
`ProjectileMotion`/`ProjectilePayload`; `move_projectiles`; `projectile_collision`; `projectile`
behavior + spawn path in execute. Scenario: a projectile crosses distance and hits (no status
yet — a plain-damage projectile), proving deferred delivery.

**3E — Demonstrators + auto-cast (baseline moves — declared).** ★
Add Fireblast/Frostbolt/Scratch/Blood Boil RONs; `self_nova` behavior; `Activation` +
`auto_cast_abilities`; per-behavior aim gate; extend the ability load list. Blood Boil goes live.
Cross-interaction, DoT-credit, and auto-cast scenarios. **Regenerate the golden baseline**
(`UPDATE_GOLDEN=1`) with a CHANGELOG entry; optionally script the campaign bot to also cast
Frostbolt/Fireblast and extend the `Snapshot` with a `statuses` count so the master net covers
status + projectiles.

**3F — Docs + gate.**
CHANGELOG "Phase 3" section; `docs/testing.md` Phase-3 scenario list; mark architecture-plan
§8 Phase-3 items done (declarative schema, generic effects, deferred hook registry & stat sheet).
Final `/compat-check`; classify the baseline move as DECLARED.

---

## 6. Validation & testing suite

Per `docs/testing.md`: one mechanic per golden scenario; cross-system drift lives in the master.

### 6.1 Unit tests (`src/**` `#[cfg(test)]`)
- `StatusEffectDef` RON round-trip for all six files (parse via `ron::de`, assert key fields) —
  mirrors the ability/talent RON tests.
- `EffectSpec` RON round-trip for death_strike / fireblast / frostbolt / scratch / blood_boil.
- Stacking: `RefreshOnReapply` (single instance, timer reset), `StackCapped(n)` (caps count),
  `StackUnlimited` — pure helper over a synthetic def set.
- Cross-tag matching (pure): event tags × active defs → removal set.
- `resolve_actor_status` math (pure): instance set → net `move_speed_mult`, `damage_taken_mult`,
  `Immobilized`.
- `MeleeCone`/`SelfNova` targeting (pure `CastOutcome` membership) + projectile circle-overlap.

### 6.2 Golden scenarios (`tests/status.rs`, `tests/projectile.rs`)
1. **bleed_ticks_over_time** — N ticks of D at the expected cadence over M s.
2. **bleed_refresh_on_reapply** — reapply extends duration; single instance; no double tick.
3. **frostbite_slows_enemy** — displacement ≈ 0.8× an unfrosted control over T.
4. **frostbite_amplifies_damage** — a known hit deals ×1.1.
5. **fire_removes_frostbite** — Fireblast on a frostbitten enemy: frostbite gone, slow gone,
   blaze applied.
6. **frost_removes_blaze** — Frostbolt on a blazing enemy: blaze gone, frostbite applied.
7. **blaze_fire_tick_clears_frostbite** — enemy with both; blaze's Fire tick removes frostbite
   with no special-casing.
8. **root_immobilizes_enemy** — a chasing enemy freezes for the duration, then resumes.
9. **stun_immobilizes_enemy** — freezes (suppress-abilities asserted no-op in Phase 3).
10. **dot_kill_credits_caster** — a lethal bleed tick awards the player XP (LastHitBy = player).
11. **status_cleaned_up_on_target_death** — no orphaned instances after the target dies.
12. **projectile_travels_then_hits** — damage lands after travel time, not at cast; pierce count
    respected.
13. **fireblast_applies_blaze_on_impact** — status applied at impact, not at cast.
14. **blood_boil_autocasts_on_cooldown** — periodic nova damage + leech with no input, gated by
    cooldown; a cone (Death Strike) still needs aim.

### 6.3 Golden master (`tests/golden_campaign.rs`)
- Baseline **regenerated** (declared): Blood Boil auto-cast now contributes damage/leech once the
  bot reaches L2/3. Reproducibility tripwire (`campaign_is_reproducible_within_a_build`) must
  still pass — no `thread_rng` in any new gameplay system; DoT/status carry no RNG.
- Recommended enrichment: teach the bot to also fire Frostbolt/Fireblast on cooldown and add a
  `statuses: usize` column to `Snapshot`, so the master exercises projectiles + status + cross
  interaction drift. (Both are declared baseline changes, landed with 3E.)

### 6.4 Compat gate
`/compat-check` after 3A (expect no diff), and after 3E (expect exactly the declared Blood-Boil
/ bot-script diff). Any other movement = regression; bisect with the focused scenarios.

---

## 7. Deferred — with the trigger that revives each

| Deferred | Revived by |
|---|---|
| General actor stat sheet (crit %, attack speed, generic move speed) | its own mini-phase; scenarios exist now for the *status-specific* slow/amp only |
| `StatusHookRegistry` (code-driven `on_*` hooks) | first talent that needs code on apply/tick/remove (Phase 4 Mage/Druid) |
| Status-consuming talent hooks (Ferocious Bite eats bleed, frost-charge-on-frostbite-death, holy-mark synergies) | the owning class (Phase 4 / 9) |
| Shields / absorbs; forced movement (knockback, grip) | their abilities (Phase 4+) |
| `suppress_abilities` live effect | enemy abilities (Phase 5) / a player-stun source |
| Class & stance binding of Fireblast/Frostbolt/Scratch/Blood Boil | Phase 4 (HeroDef + stance) |
| Status VFX (blaze glow, frost tint) | presentation pass; logic emits data only, headless-safe |

---

## 8. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Generic-effect refactor perturbs Death Strike numbers → baseline drift | 3A is a *pure* refactor with a byte-identical gate before any content lands; damage/leech math copied verbatim |
| Ordering bugs from the extended set chain (DoT/removal/stat lag) | timeline (§2.6) pinned explicitly with `.after/.before`; each latency covered by a focused scenario |
| Status instance leak on enemy death | `ChildOf` recursive despawn + defensive `despawn_orphaned_status` + scenario #11 |
| Projectile collision nondeterminism (iteration order, float) | single-threaded sim, stable query order, `already_hit` de-dup; reproducibility tripwire guards it |
| Pulling Mage abilities forward blurs the Phase-3/4 line | they ship *unbound* (no HeroDef/stance); only the status/projectile/effect machinery is Phase 3 |
| Auto-cast + aim-gate change alters Death Strike's "no cast before aim" | gate becomes per-behavior `needs_aim`; cone still waits for aim and does not burn cooldown when aimless; covered by scenario #14 |

---

_End of Phase 3 plan. Proceed 3A → 3F, gating on `/compat-check` at each ★._

---

## 9. As-built notes (completed 2026-07-05)

Phase 3 landed as planned with a few deliberate deviations, all captured in the CHANGELOG:

- **`apply_velocity` fold-in instead of a separate `apply_movement_status` system.** The plan's
  separate system would have compounded with the enemy-AI velocity lerp (multiplying the *stored*
  velocity feeds back into the next lerp). Instead `apply_velocity` scales its integration *step*
  by the generic `MoveSpeedModifier` and skips when `Immobilized`. This reads generic core
  components (not status types), avoids the feedback bug, adds no new system, and — being identical
  math for status-free entities in a single-threaded sim — kept 3C's baseline unperturbed.
- **Declarative status schema over the scaffold's hook-first sketch** (as recommended in §2.2):
  the six built-ins are pure data; the `StatusHookRegistry` is deferred until the first
  code-driven effect (Phase 4 Mage/Druid talents).
- **Status instances are plain top-level entities** (`target` field, no `ChildOf`) with an
  orphan-sweep, mirroring `AbilityInstance` — Bevy 0.16's `despawn()` is non-recursive, so the
  hierarchy would not have auto-cleaned them anyway.
- **Demonstrators + Blood Boil** landed as unbound `*.ability.ron` (fireblast/frostbolt/scratch in
  3D; blood_boil live in 3E) — no class/stance wiring, as scoped.
- **Golden baseline regenerated three times**, each declared: 3B and 3D were provably benign
  sub-unit position drift from combat-schedule reordering (verified px/py-only across all
  snapshots); 3E was the real Blood Boil auto-cast change, bundled with the master enrichment (bot
  casts Frostbolt; `statuses` snapshot column).
- **Correction to §2.3 as planned:** `suppress_abilities` is parsed from RON and asserted in the
  unit tests, but is NOT resolved into a per-actor component — `resolve_actor_status` folds only
  move/damage/immobilize. Phase 5 must add both the resolved component and its consumer (the
  enemy/player cast gate) when stuns become reachable in play.

**Known follow-up (not Phase 3):** ~~the golden master's player position is sensitive to the
single-threaded tie-break of the *loose* movement systems~~ **Done in Phase 3.1** (2026-07-05):
`MovementSet::{Intent, Integrate}` now pins the movement pipeline ahead of `CombatSet::Damage`.
The pin happened to match the prevailing tie-break, so the baseline did not even move.

---

## 10. Phase 3.1 hardening (post-review, 2026-07-05)

A review pass over the as-built Phase 3 produced a follow-up batch — see the CHANGELOG
"Phase 3.1" section for full detail. In terms of THIS plan:

- §6.1/§6.2 promises delivered late: StackCapped/StackUnlimited scenarios (synthetic defs via
  `Sim::insert_status_def`), pierce scenarios (#12's pierce clause), orphan reaping (#11).
- Bug found & fixed in §2.2's apply path: same-frame double application (Commands-deferred
  spawns invisible to later events in the same frame) could duplicate a RefreshOnReapply
  instance or overshoot a StackCapped cap.
- §2.6's freeze timeline gained a rule: combat-resolution events pending when an overlay opens
  are preserved (buffers advance only during InRun) instead of expiring; locked by
  `tests/freeze.rs`.
- §2.4's collision now reads a logic `Hurtbox` component instead of `EnemyAppearance` (the
  presentation data); the player carries one too, ready for Phase 5 enemy shots.
