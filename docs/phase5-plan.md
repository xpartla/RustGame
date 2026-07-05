# Phase 5 Implementation Plan — Enemy Ability System + AI Registry + Faction-Aware Engine

_Written 2026-07-05 against `main` @ `c4a480a` (Phases 0–4 + testing infra complete).
Companion to `docs/architecture-plan.md` (§3.9 enemy/AI, §7 phase plan, §8.1(7)/(8) + §8.5
amendments) and `docs/testing.md`. As-built notes go in §9._

---

## 0. Decisions locked for this phase

Three consequential decisions were resolved with the project owner before planning. They set
Phase 5 wider than architecture-plan §7's three bullets (which describe only the "focused" core).

| # | Decision | Consequence |
|---|---|---|
| **D1** | **Scope = full, including a ranged caster.** Not just the §7 core (data-drive enemies + contact-melee-as-ability + AI dispatch + `suppress_abilities`) — also a **ranged enemy** that fires a projectile at the player. | Forces the faction-aware engine end-to-end (an enemy projectile must hit the *player*). First non-chaser enemy; exercises the ability engine from the hostile side. |
| **D2** | **Enemy scaling = data-only model.** Add a scaling model to `EnemyDef` data + a pure resolver + one generic modifier component, but **no live driver** (there is no act/encounter depth until Phase 7). | Satisfies "enemy scaling in data" (the balance-Stage-3 prerequisite, testing.md) and is unit-tested, while staying **baseline-neutral at spawn** (depth = 0 ⇒ base stats). |
| **D3** | **Faction-aware, unified execution.** Introduce a `Faction` concept; generalize target-gathering + projectile collision so any caster hits the opposing faction. Enemies get `AbilityInstance` children and flow through the **same** `execute_ready_abilities` / `auto_cast_abilities` path as the player. | Pays down the "later this becomes faction-aware" TODO the engine already carries; one engine for both sides; clean footing for future ranged/boss enemies. |

**Deviation flagged for review (not a blocking question):** the scaffold's
`AiBehaviorRegistry` + `EnemyAiHook` trait (a `&mut World`-free hook, `todo!()` stubs) is a poor
fit for movement AI, which fundamentally needs world access (`FlowField`, player position,
`Velocity`/`Facing` writes). Phase 5 replaces it with an **`AiBehavior` component enum + plain
Bevy systems** selected by that enum — idiomatic, world-capable, and no less extensible (a new AI
= one enum variant + one system, exactly the plan's "small code hook" intent, §5). The
content-extensibility axis the *registry* pattern was meant to serve is already covered by the
ability `BehaviorRegistry`. This mirrors Phase 3 replacing the scaffold's hook-first status sketch
with a declarative model. If the owner prefers the literal trait registry, that is a larger lift —
say so before 5B.

---

## 1. Scope

### In scope
1. **Faction-aware engine (D3)** — a `Faction` component; target-gathering and projectile
   collision resolve by opposing faction instead of the hardcoded `With<Enemy>`.
2. **`EnemyDef` RON loader** — `EnemyDef` becomes a real `DefAsset` (`.enemy.ron`), loaded via
   the generic `register_def_library::<EnemyDef>()`. `enemy/archetypes.rs` (`EnemyArchetype`,
   `archetypes()`, `pick()`) is **deleted**; the three archetypes become RON files.
3. **Contact melee as a first-class ability (D3)** — the hardcoded `enemy_attack` system +
   `AttackStats`/`AttackCooldown` are replaced by an auto-cast `contact_melee` ability the enemy
   carries as an `AbilityInstance`, delivering damage through the shared effect applier.
4. **AI dispatch** — an `AiBehavior` component (`MeleeChaser | RangedCaster | Stationary`) sourced
   from `EnemyDef.ai_behavior`; the existing flow-field follower is gated to `MeleeChaser`.
5. **Ranged caster (D1)** — one ranged enemy (working name **"spitter"**): approaches to a
   stand-off range, stops, faces the player, and auto-casts a projectile that hits the player via
   the faction-aware collision path.
6. **Enemy scaling data model (D2)** — a `scaling` block on `EnemyDef`, a pure
   `resolve_enemy_stats(def, depth)` resolver, and a generic `DamageDealtModifier` component (the
   symmetric partner of `DamageTakenModifier`). Depth = 0 at every live spawn ⇒ neutral.
7. **`suppress_abilities` wiring (§8.5 debt)** — `resolve_actor_status` folds it into a new
   `AbilitiesSuppressed` marker; `auto_cast_abilities`, `execute_ready_abilities`, and the hero
   input/stance systems skip a suppressed caster.
8. **Full test suite + docs.**

### Out of scope (explicitly deferred — see §7)
`ThemeDef` loader + theme-driven / encounter spawning (Phase 7); `EnemyRarity::Elite`/boss spawn
logic (Phase 7); multi-phase **boss AI** (Phase 9); a **live** enemy-scaling driver (needs act
depth — Phase 7); enemy status/DoT application beyond plain physical damage (content, Phase 9);
**AMZ** projectile-blocking zones (Phase 6+); forced movement / knockback (later); projectiles
still pass through walls (owner-accepted, 2026-07-05).

---

## 2. Architecture

### 2.1 Faction-aware targeting (D3) — the load-bearing change

Today the whole engine is hardwired player→enemy: `execute_ready_abilities` gathers candidates as
`Query<…, With<Enemy>>`, and `projectile_collision` only tests `With<Enemy>`. The behavior code
already flags it: *"Phase 1: every `Enemy`. Later this becomes faction-aware."*

```rust
// core/components.rs (new)
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Faction { Friendly, Hostile }
```

- `spawn_player` inserts `Faction::Friendly`; `enemy` spawns insert `Faction::Hostile`.
  (Player-side summons — Companion — will be `Friendly`; enemy summons `Hostile`.)
- **Target gathering** (`execute_ready_abilities`): gather **two** candidate lists once per frame
  (friendlies, hostiles) from `Query<(Entity, &WorldPosition, &Faction)>`. Per cast, hand the
  behavior the list **opposing the caster's faction** (read from the owner). `AbilityContext.enemies`
  is renamed to `AbilityContext.targets` (pure rename; `MeleeCone`/`SelfNova`/`Projectile` read it
  unchanged). For a player caster, opposing = hostiles = today's `With<Enemy>` set (same members,
  same query order) ⇒ **neutral**.
- **Projectile collision**: `ProjectilePayload` gains `target_faction: Faction`, baked at spawn as
  the opposite of the caster's faction. `projectile_collision` queries
  `Query<(Entity, &WorldPosition, &Hurtbox, &Faction)>` and only collides matching factions. A
  player's Frostbolt bakes `Hostile`, hits the same enemy set in the same order ⇒ **neutral**; an
  enemy's bolt bakes `Friendly` and can finally hit the player (the `Hurtbox` on the player has
  existed since Phase 3.1, "ready for Phase 5 enemy shots").

Neutrality argument: every enemy uniformly gains one component (`Faction`), so enemies stay in a
single archetype iterated in the same entity order; the candidate **set and order** for player
casts is unchanged; no system moves between sets. 5A therefore does not move the golden baseline.

### 2.2 `EnemyDef` data-drive (replaces `archetypes.rs`)

`EnemyDef` already exists as a Bevy `Asset` but lacks `Deserialize` + `DefAsset`. Add both (mirror
`HeroDef`), extension `.enemy.ron`, and register via `register_def_library::<EnemyDef>()`. The
schema is reshaped so one RON file is the single source of truth per enemy — stats, presentation,
AI, abilities, scaling:

```rust
// enemy/assets.rs
pub struct EnemyDef {
    pub id: EnemyId,
    pub display_name: String,
    pub rarity: EnemyRarity,                 // Common | Elite | MapBoss | ActBoss (spawn roles: Phase 7)
    pub base_stats: EnemyBaseStats,          // max_health, move_speed, size_radius
    pub appearance: EnemyAppearanceDef,      // { shape: EnemyShape, color: (f32,f32,f32) }  → EnemyAppearance
    pub spawn_weight: u32,                    // ambient weighted pick (was archetype.weight)
    pub ai_behavior: AiBehaviorId,            // "melee_chaser" | "ranged_caster" | "stationary"
    #[serde(default)] pub preferred_range: f32, // ranged stand-off distance (0 = melee/none)
    pub abilities: Vec<AbilityId>,            // CHANGED: id refs (were inline EnemyAbilityDef)
    pub xp_value: u32,
    pub drop_table: String,                   // placeholder string (Phase 7/9)
    #[serde(default)] pub scaling: EnemyScaling, // §2.5
}
```

- **Presentation stays data, not a mesh.** `appearance` carries `EnemyShape` + an `(r,g,b)` tuple
  (like `ThemeDef.ambient_tint`); the spawn path copies it into the presentation-only
  `EnemyAppearance` component (built into a mesh by `attach_enemy_visuals` on the Windows build).
  The logic sim never reads it, so appearance values are **baseline-irrelevant**. `EnemyShape`
  gains `Deserialize`.
- **Spawn path.** A new `spawn_enemy_from_def(commands/world, def, grid, depth)` spawns the enemy
  entity **and its `AbilityInstance` children together** (so contact melee is queryable the very
  next frame — see §2.3 timing). The ambient `spawn_enemy_over_time` weighted-picks an `EnemyDef`
  from the loaded library (still on `thread_rng`, still paused in scenarios — docs/testing.md);
  `Sim::spawn_enemy(id, tile)` / `spawn_grunt` and the golden campaign spawn by **id**.
- Port `grunt`/`runner`/`brute` to `assets/enemies/*.enemy.ron` with **byte-identical** logic
  numbers (health/speed/damage/range/cooldown/radius/xp/weight). `grunt_placeholder.ron` is renamed
  and reshaped.

### 2.3 Contact melee as an auto-cast ability

`enemy_attack` (proximity contact + `AttackStats`/`AttackCooldown`) is replaced by an auto-cast
`contact_melee` ability so contact damage flows through the one shared applier:

- New `contact_melee` behavior: hits opposing-faction actors within `range` of the caster, no aim
  (`needs_aim() == false`), `primary = nearest`. Structurally a proximity nova; kept distinct from
  `self_nova` for clarity.
- Enemy ability RON files — one per (enemy, ability), so per-enemy numbers survive:
  `grunt_contact` (5/28/1.0s), `runner_contact` (3/24/0.7s), `brute_contact` (12/32/1.6s), all
  `activation: AutoCast`, effect `Damage(amount:"damage", tags:[Physical], target: AllHits)`.
  Registered in `AbilityDef::MANIFEST`.
- **Cadence fidelity (this is the one baseline-moving step — see §6).** The prototype's contact
  attack "charges while approaching, hits immediately on contact, and does not waste its swing out
  of range." Two mechanisms preserve it exactly:
  1. **Spawn ability instances *with* the enemy** (§2.2), so the first `auto_cast → execute` frame
     already sees a ready contact instance — first hit lands the frame after spawn, as before.
  2. **Whiff gate.** `AbilityBehavior::consumes_cooldown_on_whiff() -> bool` (default `true`);
     `contact_melee` overrides to `false`. In `execute_ready_abilities`, when a behavior resolves
     **zero hits and no projectile** and opts out, `break` **without** resetting the cooldown — so
     an out-of-range enemy stays charged and strikes the instant it enters range. `melee_cone`
     (Death Strike), `self_nova` (Blood Boil), and `projectile` keep the default `true`, so their
     whiff-consumes-cooldown behavior is **unchanged** ⇒ neutral for existing content.

  Result: identical per-hit cadence and identical first-hit frame. The only residual difference is
  that enemy contact `DamageEvent`s are now written from `execute_ready_abilities` rather than
  `enemy_attack` — a **write-order** change within `CombatSet::Damage` that may perturb `f32`
  accumulation in the golden master (the documented "combat reorder ⇒ declared regen" case, §6).

### 2.4 AI dispatch (`AiBehavior` enum, plain systems)

```rust
// enemy/components.rs
#[derive(Component, Clone, Copy)]
pub enum AiBehavior { MeleeChaser, RangedCaster, Stationary }
```

Set at spawn from `EnemyDef.ai_behavior`. The scaffold's `AiBehaviorRegistry`/`EnemyAiHook`/
`EnemyAiContext` (`todo!()`) are removed (§0 deviation).

- **`MeleeChaser`** — the existing `enemy_follow_flow_field` + `update_enemy_facing`, now filtered
  to `AiBehavior::MeleeChaser`. All ported archetypes are chasers ⇒ same set ⇒ neutral.
- **`RangedCaster`** — new `ranged_caster_ai` (in `MovementSet::Intent`): while
  `dist(player) > preferred_range`, steer via the flow field (same lerp as the chaser); at/inside
  `preferred_range`, zero the velocity (stand and shoot). **Always face the player** (not velocity),
  so the aim-dependent `projectile` behavior can fire while stationary. Its projectile ability
  (`AutoCast`) then fires on cooldown through the normal path.
- **`Stationary`** — velocity stays zero; faces the player; casts on cooldown. (Minimal; ready for
  turret-like enemies.)

### 2.5 Enemy scaling — data-only model (D2)

Scaling is a property of enemy *data*, applied at spawn against a **depth** the encounter system
will supply in Phase 7. Until then every live spawn passes `depth = 0`, so nothing changes.

```rust
// enemy/assets.rs
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct EnemyScaling {
    pub health_per_depth: f32,   // additive fraction per depth step (e.g. 0.15 = +15%/depth)
    pub damage_per_depth: f32,
    pub xp_per_depth: f32,
}
```

- **Pure resolver** `resolve_enemy_stats(def, depth) -> ResolvedEnemyStats` applies
  `base * (1.0 + growth * depth)` to health, xp, and a **damage multiplier** (move speed and radius
  are not scaled). Unit-tested directly.
- **Delivery.** The spawn path sets `Health`/`XpReward` from the resolved values and inserts a
  generic `DamageDealtModifier(f32)` on the enemy **iff** the damage multiplier ≠ 1.0.
  `apply_damage` multiplies `event.amount` by the **source's** `DamageDealtModifier` (default 1.0
  when absent) — the mirror of the existing target-side `DamageTakenModifier`. So scaled enemies hit
  harder without duplicating damage numbers per depth.
- **Neutrality.** At `depth = 0`, resolved == base and no `DamageDealtModifier` is inserted, so the
  component is absent everywhere in shipped play and `apply_damage` multiplies by 1.0 ⇒ byte-identical.
  A balance sweep (Stage 3) can spawn at `depth > 0` to exercise the curve.

### 2.6 `suppress_abilities` wiring (§8.5 debt)

`stun.status.ron` sets `suppress_abilities: true`, parsed since Phase 3 but never consumed.

- New marker `AbilitiesSuppressed` (core/components.rs), reconciled by `resolve_actor_status`
  exactly like `Immobilized` (insert when any active status suppresses, remove when none).
- **Gates:** `auto_cast_abilities` and `execute_ready_abilities` skip a caster whose owner has
  `AbilitiesSuppressed`; the hero `resolve_input_to_ability` and `handle_stance_swap` skip a
  suppressed player. A stunned actor (enemy or player) cannot cast, auto-cast, or swap stance.
- **Neutrality.** No shipped ability applies `stun` to anyone, and the golden campaign never stuns,
  so the marker is never present ⇒ the gates never trigger ⇒ baseline unchanged. Reachable now via
  `Sim::apply_status(target, …, "stun", 1)` for the scenario.

### 2.7 Frame timeline (unchanged skeleton)

No new system sets. New systems slot into the existing chain:

```
MovementSet::Intent      : player_input, flow-field rebuild, enemy_follow_flow_field (MeleeChaser),
                           ranged_caster_ai (RangedCaster), update_enemy_facing              ★ new AI
MovementSet::Integrate   : apply_velocity → world_to_grid
CombatSet::Damage        : tick_ability_cooldowns → auto_cast_abilities → execute_ready_abilities
                           (now casts for BOTH factions; enemy contact/bolt included)         ★
                           move_projectiles → projectile_collision (faction-aware)            ★
                           [enemy_attack DELETED]
CombatSet::Apply         : apply_damage (× DamageTakenModifier[target], × DamageDealtModifier[source]) ★
StatusSet::Tick/CrossInteract : … resolve_actor_status (now also → AbilitiesSuppressed)       ★
CombatSet::Death         : enemy_death, player_death
```

---

## 3. File-level change map

| Area | File(s) | Change |
|---|---|---|
| Faction | `core/components.rs` | `Faction` enum; `DamageDealtModifier`; `AbilitiesSuppressed` |
| Faction | `player/systems/spawn_player.rs`, `enemy/systems/spawner.rs` | insert `Faction` on player / enemies |
| Faction | `ability/behavior.rs` | rename `AbilityContext.enemies` → `targets`; add `consumes_cooldown_on_whiff`; `contact_melee` behavior |
| Faction | `ability/systems/execute.rs` | two-list faction gather; opposing list per cast; whiff gate; suppress gate |
| Faction | `projectile/components.rs`, `projectile/systems/motion.rs` | `ProjectilePayload.target_faction`; collide by faction |
| Enemy def | `enemy/assets.rs` | `Deserialize` + `DefAsset` (`.enemy.ron`); reshape `EnemyDef` (`appearance`, `spawn_weight`, `preferred_range`, `abilities: Vec<AbilityId>`, `scaling`); `EnemyAppearanceDef`; `resolve_enemy_stats`; parse + scaling unit tests |
| Enemy def | `enemy/archetypes.rs` | **deleted** (`EnemyArchetype`/`archetypes()`/`pick()`) |
| Enemy def | `enemy/components.rs` | `EnemyShape` (+ `Deserialize`) rehomed; `AiBehavior`; drop `AttackStats`/`AttackCooldown` |
| Enemy | `enemy/systems/spawner.rs` | `spawn_enemy_from_def` (enemy + ability instances); weighted pick from library |
| Enemy | `enemy/systems/attack.rs` | **deleted** (`enemy_attack`) |
| Enemy | `enemy/systems/follow_flow_field.rs`, `update_enemy_facing.rs` | gate on `AiBehavior::MeleeChaser` |
| Enemy | `enemy/systems/ranged_caster.rs` (new) | approach-to-range + stop + face-player |
| Enemy | `enemy/behavior.rs` | **deleted / gutted** (registry scaffold → enum, §0) |
| Enemy | `enemy/plugin.rs` | register `EnemyDef`; new AI systems; drop `enemy_attack` |
| Core | `core/systems/apply_damage.rs` | multiply by source `DamageDealtModifier` |
| Status | `status/systems/resolve.rs` | fold `suppress_abilities` → `AbilitiesSuppressed` |
| Hero | `hero/systems/input_slot.rs`, `stance.rs` | skip a suppressed player |
| Content | `assets/enemies/{grunt,runner,brute,spitter}.enemy.ron` | ported archetypes + one ranged enemy |
| Content | `assets/abilities/{grunt,runner,brute}_contact.ability.ron`, `spitter_bolt.ability.ron` | enemy abilities (AutoCast) |
| Sim | `src/sim/mod.rs` | `spawn_enemy(id,…)`/`spawn_grunt` by id; `spawn_enemy_at_depth`; `enemy_ability_ids`; `faction`; await `EnemyLibrary` |
| Tests | `tests/enemy.rs` (new); `tests/combat.rs` (contact-cadence mechanism) ; `tests/golden_campaign.rs` (id spawns) | scenarios + campaign spawn migration |

---

## 4. Content

### 4.1 Enemies (`*.enemy.ron`)
| id | rarity | hp | speed | radius | ai | pref. range | abilities | xp | weight | notes |
|---|---|---|---|---|---|---|---|---|---|---|
| grunt | Common | 10 | 15 | 12 | melee_chaser | – | [grunt_contact] | 3 | 6 | ported Grunt (byte-identical) |
| runner | Common | 5 | 28 | 9 | melee_chaser | – | [runner_contact] | 2 | 3 | ported Runner |
| brute | Elite* | 30 | 8 | 18 | melee_chaser | – | [brute_contact] | 8 | 1 | ported Brute (*rarity label only; spawn roles are Phase 7) |
| spitter | Common | 8 | 12 | 11 | ranged_caster | 140 | [spitter_bolt] | 4 | – | new ranged demonstrator (working name) |

`scaling` on each is a placeholder (e.g. `health_per_depth: 0.15, damage_per_depth: 0.12,
xp_per_depth: 0.10`); inert at depth 0. `appearance` mirrors the old archetype color/shape.

### 4.2 Enemy abilities (`*.ability.ron`, all `activation: AutoCast`)
| id | behavior | params | effects |
|---|---|---|---|
| grunt_contact | contact_melee | damage 5, range 28, cooldown 1.0 | Damage(Physical, AllHits) |
| runner_contact | contact_melee | damage 3, range 24, cooldown 0.7 | Damage(Physical, AllHits) |
| brute_contact | contact_melee | damage 12, range 32, cooldown 1.6 | Damage(Physical, AllHits) |
| spitter_bolt | projectile | damage 6, speed 260, radius 8, range 320, pierce 0, cooldown 1.6 | Damage(Physical, PrimaryHit) |

Registered behaviors after Phase 5: `melee_cone`, `self_nova`, `projectile`, **`contact_melee`**.
(`dropped_zone`/`orbiting`/`summon`/… still unregistered → inert.)

---

## 5. Implementation sequence (each step is independently `/compat-check`-able)

Ordered so behavior stays **unchanged** until the single deliberate baseline move in 5B.

**5A — Faction-aware engine (behavior-neutral).** ★ Gate: **baseline byte-identical.**
`Faction` on player/enemies; two-list gather + opposing-per-cast (`AbilityContext.targets`);
faction-baked projectile collision. No system moves sets; candidate sets/order unchanged.

**5B — EnemyDef data-drive + contact-melee-as-ability + scaling model (declared benign regen).** ★
Delete `archetypes.rs`/`enemy_attack`/`AttackStats`; reshape+register `EnemyDef`; port the three
archetypes + their `*_contact` abilities; spawn enemies **with** their ability instances; whiff
gate; `AiBehavior` enum + chaser gating; scaling data + `resolve_enemy_stats` + `DamageDealtModifier`;
migrate sim/campaign to id spawns. Gate: the only expected diff is a **write-order** `f32` shift in
the master (enemy contact damage now from `execute`). **Verify the diff is benign** — every
discrete column (`enemies`, `level`, `xp`, `abilities`, `talents`, `statuses`) identical, only
`hp`/`px`/`py` micro-shift — then `UPDATE_GOLDEN=1` with a CHANGELOG entry. If any discrete column
moves, it is a regression: bisect with the focused scenarios.

**5C — Ranged caster (neutral to the master).** ★ Gate: **baseline byte-identical.**
`spitter` EnemyDef + `spitter_bolt` + `ranged_caster_ai` + enemy projectiles hitting the player.
The spitter is **not** added to the golden campaign, so the master is untouched; covered by
scenarios only.

**5D — `suppress_abilities` wiring (neutral).** ★ Gate: **baseline byte-identical.**
`AbilitiesSuppressed` + `resolve_actor_status` fold + cast/auto-cast/input gates. Marker never
present in the master ⇒ no movement.

**5E — Docs + final gate.** This file's §9 as-built; CHANGELOG "Phase 5"; architecture-plan §8.5
(retire the `suppress_abilities` row) + new §8.7 "Phase 5 delivered" + §8.1(7)/(8) status;
docs/testing.md Phase-5 scenarios; Mechanics.md working-name note for the spitter; repo CLAUDE.md.
Full `/compat-check`; classify the 5B baseline move as DECLARED.

---

## 6. Validation & testing suite

### 6.1 Unit tests (`src/**` `#[cfg(test)]`)
- `enemy/assets.rs` — parse `grunt/runner/brute/spitter.enemy.ron` (stats, ai, abilities, scaling).
- `resolve_enemy_stats` — depth 0 == base; depth N grows health/damage/xp by the declared fractions.
- `ability/behavior.rs` — `contact_melee` targeting (in-range hits, out-of-range empty, primary =
  nearest); `consumes_cooldown_on_whiff` defaults/override.
- Faction selection (pure) — a caster's opposing list excludes same-faction actors.
- `AiBehavior` mapping from `ai_behavior` string.

### 6.2 Golden scenarios (`tests/enemy.rs`)
1. **enemy_def_spawns_with_declared_stats** (testing.md DoD) — `spawn_enemy("grunt")` ⇒ Health 10,
   MoveSpeed 15, Hurtbox 12, XpReward 3, owns a `grunt_contact` `AbilityInstance`, `Faction::Hostile`.
2. **grunt_contact_attack_cadence** — via the ability path: first hit immediate on contact, then
   once/1.0s (the frozen prototype cadence; the assertions from `tests/combat.rs` preserved).
3. **enemy_contact_hits_player_not_other_enemies** — faction: an in-range grunt damages the player,
   never an adjacent grunt.
4. **ranged_caster_stops_at_range_and_shoots** — a spitter approaches to `preferred_range`, halts,
   faces the player, and its bolt travels and damages the **player**.
5. **enemy_projectile_ignores_hostiles** — the spitter bolt passes other enemies, hits only the player.
6. **player_abilities_still_only_hit_enemies** — regression: Death Strike / Frostbolt hit enemies,
   the player is unharmed by its own casts.
7. **suppressed_caster_cannot_cast** — `apply_status(enemy,…,"stun")` (or player): no cast while the
   marker is present; casting resumes after expiry.
8. **enemy_scaling_scales_health_and_damage** — `spawn_enemy_at_depth("grunt", …, depth)` ⇒ scaled
   Health and a contact hit scaled by `DamageDealtModifier`.

### 6.3 Golden master
Regenerated **once**, in 5B, as a DECLARED benign change (contact damage now flows through the
ability path — a `CombatSet::Damage` write-order shift, the documented reorder case). The bot,
waves, and enemy roster (grunt + brute) are **unchanged**; the ranged spitter is deliberately kept
out of the master to bound baseline churn (ranged coverage lives in scenarios). The reproducibility
tripwire must still pass — no `thread_rng` enters any gameplay system; enemy AI/abilities carry no RNG.

### 6.4 Compat gate
`/compat-check` at every ★. 5A/5C/5D expect **no** diff; 5B expects **exactly** the benign
write-order diff. Any discrete-column movement, or any drift in 5A/5C/5D, is a regression.

---

## 7. Deferred — with the trigger that revives each

| Deferred | Revived by |
|---|---|
| `ThemeDef` loader + theme/encounter-driven spawning | Phase 7 (act graph + rooms) |
| `EnemyRarity::Elite` / boss spawn-role logic | Phase 7 |
| Multi-phase **boss AI** | Phase 9 (boss design) |
| **Live** enemy-scaling driver (depth from act/map index) | Phase 7 (an axis to scale against) |
| Enemy status/DoT application (poison bolts, chill, …) | enemy content pass (Phase 9) |
| **AMZ** projectile-blocking zone | Phase 6+ (zones) |
| Forced movement / knockback (shockwave, grip) | its abilities (later) |
| `execute_ready_abilities` resolve/apply split | first code-driven hook (the whiff gate is a flag, not a hook — still none) |
| Projectiles ignore walls | owner-accepted (2026-07-05); revisit at Mage playtest |

---

## 8. Risks & mitigations

| Risk | Mitigation / expected outcome |
|---|---|
| 5B moves the master beyond benign write-order noise | Isolate to 5B; verify every **discrete** column identical (only hp/px/py micro-shift); if a discrete column moves, treat as regression and bisect with §6.2. |
| Contact cadence changes (first hit delayed / cooldown wasted) | Spawn ability instances **with** the enemy (no `Added` race) + the whiff gate; `grunt_contact_attack_cadence` locks the exact frames. |
| Faction gather perturbs player casts (set/order) | Uniform single component on enemies keeps one archetype + stable order; candidate set identical; 5A gated on an unchanged baseline. |
| `DamageDealtModifier` accidentally non-neutral at depth 0 | Component inserted only when the multiplier ≠ 1.0; depth 0 ⇒ absent ⇒ `apply_damage` × 1.0. |
| Suppress gate over-broad (freezes normal play) | Marker only present when a `suppress_abilities` status is active; none is applied in shipped content or the master. |
| Ranged AI feels wrong (jitter at range boundary, can't aim while still) | Face the player independent of velocity; stop with hysteresis at `preferred_range`; Windows playtest (headless can't judge feel). |
| Registry deviation (§0) rejected by owner | Flagged before 5B; the enum swap is localized to `enemy/` and reversible. |

---

## 9. As-built notes (completed 2026-07-05)

Phase 5 landed as planned across the five steps, with two pleasant surprises and one confirmed
deviation:

- **5B was fully baseline-neutral — no regeneration.** The plan budgeted for a *declared benign*
  golden-master regen (enemy contact damage moving from `enemy_attack` to `execute` changes the
  `DamageEvent` write order). In practice `campaign_matches_golden_baseline` passed **byte-identical**:
  the cadence reproduction (spawn ability instances *with* the enemy + the `consumes_cooldown_on_whiff`
  opt-out) lands the same damage on the same frames, and the values are exact small floats that
  survive the 2-decimal snapshot rounding. So the **entire phase moved the baseline zero times.**
- **The scaffolds were already dead.** `EnemyDef`/`ThemeDef` (`enemy/assets.rs`) and the
  `AiBehaviorRegistry` (`enemy/behavior.rs`) were never declared in `enemy/mod.rs` — uncompiled. So
  wiring `EnemyDef` was fresh code (no `Deserialize` to retrofit on a live type), and deleting
  `behavior.rs` cost nothing. The §0 AI-registry deviation was therefore free: `AiBehavior` is a new
  component enum, not a replacement of live code.
- **AI dispatch = component enum (confirmed deviation from §3.9).** Flow-field steering needs
  `Res<FlowField>` + `Velocity`/`Facing` writes, which the scaffold's `&mut World`-free
  `EnemyAiHook` could not express. `AiBehavior { MeleeChaser | RangedCaster | Stationary }` +
  plain systems is idiomatic and gated the melee systems to `MeleeChaser` neutrally. The owner did
  not object before 5B.
- **Contact melee via a whiff gate.** `AbilityBehavior::consumes_cooldown_on_whiff()` (default
  `true`, `contact_melee` → `false`) is the one execute-path change; it keeps Death Strike / Blood
  Boil / projectiles identical while giving contact melee its "charge while approaching, strike on
  contact" cadence. Modelled contact melee as **one `.ability.ron` per enemy** (`grunt_contact`, …)
  rather than inline `EnemyAbilityDef`, so `EnemyDef.abilities` is `Vec<AbilityId>` and the ability
  engine stays single-pathed.
- **Enemy scaling delivered via a symmetric `DamageDealtModifier`.** Rather than bake per-depth
  damage numbers, scaling inserts a source-side damage multiplier (mirror of `DamageTakenModifier`),
  read in `apply_damage`. Neutral at depth 0 (component absent). `resolve_enemy_stats` is pure and
  unit-tested; the sim exposes `spawn_enemy_at_depth`.
- **`suppress_abilities`** folded into `resolve_actor_status` beside `Immobilized` and gated in
  auto-cast/execute + the two hero systems. Neutral (no shipped stun applier).
- **Debug flash repointed** from the removed `AttackCooldown` to the enemy's contact `AbilityCooldown`
  (presentation-only). `EnemyShape` rehomed to `enemy/components.rs` with `Deserialize`.
- **Tests: 94 passing** (was 84): +3 unit (`EnemyDef` parse ×2, scaling math) and +7 golden
  scenarios (`tests/enemy.rs`). Build warning-free. **Golden baseline unchanged (no regeneration).**
- **Debt (architecture-plan §8.5/§8.7):** `suppress_abilities` **resolved**; §8.1(7) enemy scaling
  **done (data model)**; §8.1(8) enemy **projectiles done** (AMZ blocking still open). Re-filed
  deferred (phase5-plan §7): `ThemeDef`/theme spawning + `Elite`/boss roles + live scaling driver
  (Phase 7); boss AI + enemy DoT kits (Phase 9); AMZ zones (Phase 6+). Wall-collision remains
  owner-accepted.
