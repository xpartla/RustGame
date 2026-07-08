// BehaviorRegistry — the open extension point of the ability system, plus the built-in
// behavior implementations.
//
// Adding a new ability shape:
//   1. Implement AbilityBehavior for a unit struct.
//   2. Call registry.register("my_behavior", MyBehavior) in AbilityPlugin::build.
//   3. Set `behavior: "my_behavior"` in the ability's RON file.
//   No other code changes.
//
// Execution model (deliberately &mut World-free), Phase-3 generic-effect form:
//   A behavior reads an AbilityContext (owner identity, position, aim, candidate targets) and
//   returns a CastOutcome — *which* entities it hit (targeting/geometry), the origin, an optional
//   shape VFX, and (Phase 3D) an optional projectile to spawn. It does NOT decide damage / leech /
//   status — those are the ability's declarative `effects: Vec<EffectSpec>` (ability/assets.rs),
//   applied by the execute system against the CastOutcome. This keeps behaviors pure and trivially
//   testable, lets one behavior back many abilities (a fire vs. frost projectile differ only in
//   data), and matches architecture-plan §3.3 + the Phase-3 plan §2.1.
//
// Behaviors still pending (registered in their phase; until then execute_ready_abilities skips
// any ability whose behavior id is not in the registry):
//   "periodic_self_zone" — self-centred pulsing zone (later)

use bevy::prelude::*;
use std::collections::HashMap;
use crate::ability::assets::{BehaviorId, StatId};
use crate::constants::ATTACK_LIFETIME;

/// Resolved numeric parameters after the talent modifier stack has been applied.
/// Produced by resolve_params() (talent/modifier.rs).
#[derive(Debug, Clone)]
pub struct ResolvedParams(pub HashMap<StatId, f32>);

impl ResolvedParams {
    /// Returns the param value, or 0.0 if the stat is not present.
    pub fn get(&self, stat: &str) -> f32 {
        *self.0.get(stat).unwrap_or(&0.0)
    }

    /// Overwrites a param (Phase 6 — used by Pre hooks to rewrite a resolved number).
    pub fn set(&mut self, stat: &str, value: f32) {
        self.0.insert(stat.to_string(), value);
    }

    /// Multiplies an existing param by `factor` in place; no-op if the stat is absent. Used by
    /// condition hooks (e.g. `blood_boil_dnd_range` doubles `radius` inside D&D).
    pub fn scale(&mut self, stat: &str, factor: f32) {
        if let Some(v) = self.0.get_mut(stat) {
            *v *= factor;
        }
    }
}

/// A candidate target the execute system gathers up front and hands to the behavior.
/// Phase 5: faction-aware — the execute system hands each caster the actors of the *opposing*
/// faction (enemies for a player cast, the player for an enemy cast).
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub entity: Entity,
    pub pos: Vec2,
    /// Whether this actor's `AiBehavior` is `RangedCaster` (Phase 9.2 — Abomination Limb's
    /// "grip only ranged enemies" epic talent). Always `false` for non-enemy targets (the player,
    /// a Companion minion), which carry no `AiBehavior`.
    pub is_ranged: bool,
    /// Current health (Phase 9.4 — Primal Pounce's "leap toward the highest-health enemy").
    pub health: f32,
}

/// One entity a behavior resolved as hit, with its position (for follow-up geometry / VFX).
#[derive(Debug, Clone, Copy)]
pub struct HitTarget {
    pub entity: Entity,
    pub pos: Vec2,
}

/// A shape VFX a behavior wants drawn. Presentation-only; spawned by the execute system as a
/// transient hitbox-gizmo entity (reusing the prototype's Projectile + ArcHitbox + Lifetime path).
#[derive(Debug, Clone, Copy)]
pub enum VfxShape {
    Cone { radius: f32, half_angle: f32, forward: Vec2, lifetime: f32 },
}

/// A persistent ground zone a behavior wants dropped (D&D, Consecrated Ground, AMZ, Tree Conduit).
/// The behavior resolves only the drop *point*; the ability's `zone` spec (ability/assets.rs) +
/// resolved params supply the zone type, anchor, radius/duration, and any occupant effects — the
/// execute system builds the `PersistentZone` entity. Mirrors `ProjectileSpawn`.
#[derive(Debug, Clone, Copy)]
pub struct ZoneSpawn {
    /// Where the zone is dropped (caster origin for `dropped_zone`).
    pub center: Vec2,
}

/// A travelling projectile a behavior wants spawned. The execute system spawns the entity and
/// attaches the ability's baked effects (applied on impact by projectile/systems/collision.rs).
#[derive(Debug, Clone, Copy)]
pub struct ProjectileSpawn {
    pub velocity: Vec2,
    /// Collision radius of the projectile itself (added to the enemy radius at impact).
    pub radius: f32,
    /// Extra enemies it can pass through after the first hit (0 = despawn on first hit).
    pub pierce: u32,
    /// Seconds before it despawns if it hits nothing.
    pub lifetime: f32,
}

/// A forced-movement impulse a behavior wants applied to its OWN caster (dash/blink, Phase 9.1) —
/// unlike `ZoneSpawn`/`ProjectileSpawn`, which spawn a new world entity, this targets the caster
/// directly. The execute system turns it into a `core::components::ForcedImpulse` inserted on the
/// caster, resolved once at cast time.
#[derive(Debug, Clone, Copy)]
pub struct ForcedImpulseSpawn {
    pub velocity: Vec2,
    pub duration: f32,
}

/// A minion a behavior wants spawned (Phase 9.2 — Companion), at the caster's `origin` (already on
/// `CastOutcome`). Mirrors `ZoneSpawn`'s minimal-signal shape: the behavior only signals "spawn
/// here"; the ability's `summon` spec (`AbilityDef.summon`, ability/assets.rs) supplies `mimic` and
/// resolved params supply the duration — `resolve()` has no access to the `AbilityDef` itself, only
/// `ResolvedParams`, so it cannot carry a real `AbilityId` here.
#[derive(Debug, Clone, Copy)]
pub struct SummonSpawn;

/// A forced pull toward the caster requested on one gripped target (Phase 9.2 — Abomination
/// Limb). Unlike `ForcedImpulseSpawn` (which targets the caster itself), this names a specific
/// OTHER entity to yank. The execute system turns each into a `core::components::ForcedImpulse`
/// on `target`.
#[derive(Debug, Clone, Copy)]
pub struct GripSpawn {
    pub target: Entity,
    pub velocity: Vec2,
    pub duration: f32,
}

/// A request to start a multi-frame channel (Phase 9.3 — Flash of Light; later Druid Heal / Mage
/// Frost Impale reuse the same behavior). The execute system, on seeing this, does NOT apply the
/// ability's effects instantly — it inserts a `ability::components::Channeling` on the caster that
/// `ability::systems::channel::tick_channels` resolves once `cast_time` elapses. "While moving":
/// nothing in this primitive restricts caster movement during the channel; "no interrupt" (the
/// phase-9 plan's default): nothing cancels it once started, not even taking damage.
#[derive(Debug, Clone, Copy)]
pub struct ChannelSpawn {
    pub cast_time: f32,
}

/// A collectible pickup a behavior wants dropped at the caster's origin (Phase 9.4 — Druid's
/// Bloom). Mirrors `ZoneSpawn`/`SummonSpawn`'s minimal-signal shape: the behavior only signals
/// "drop one here"; the execute system builds the actual `pickup::components::PickUp` entity (the
/// `Bloom` ability has no `EffectSpec`/spec type of its own to carry a payload — the grant amount
/// is read straight from resolved params, see execute.rs's pickup-spawn handling).
#[derive(Debug, Clone, Copy)]
pub struct PickupSpawn;

/// What a behavior resolves for one cast: the targeting result the execute system applies the
/// ability's declarative `effects` against. Gameplay outcome (damage/heal/status) is data, not here.
#[derive(Debug, Clone, Default)]
pub struct CastOutcome {
    /// Caster origin, used for Caster-scoped effects (leech heal) and VFX placement.
    pub origin: Vec2,
    /// Every entity the shape hit. `EffectTarget::AllHits` maps to this.
    pub hits: Vec<HitTarget>,
    /// The nearest/first hit. `EffectTarget::PrimaryHit` maps to this.
    pub primary: Option<HitTarget>,
    /// Optional shape VFX for the presentation layer.
    pub vfx: Option<VfxShape>,
    /// Optional travelling projectile to spawn (deferred delivery — see projectile module).
    pub projectile: Option<ProjectileSpawn>,
    /// Optional persistent zone to drop (Phase 6 — D&D, Consecrated Ground, AMZ, Tree Conduit).
    /// The execute system builds the `PersistentZone` from the ability's `zone` spec + params.
    pub zone: Option<ZoneSpawn>,
    /// Optional forced-movement impulse to apply to the caster (Phase 9.1 — the Movement-slot
    /// dash/blink). `None` for every other behavior.
    pub forced_impulse: Option<ForcedImpulseSpawn>,
    /// Optional minion to spawn (Phase 9.2 — Companion). `None` for every other behavior.
    pub summon: Option<SummonSpawn>,
    /// Targets to forcibly pull toward the caster (Phase 9.2 — Abomination Limb). Empty for every
    /// other behavior.
    pub grip_targets: Vec<GripSpawn>,
    /// Optional request to start a multi-frame channel (Phase 9.3). `None` for every other
    /// behavior.
    pub channel: Option<ChannelSpawn>,
    /// Optional request to drop a collectible pickup at the caster's origin (Phase 9.4 — Bloom).
    /// `None` for every other behavior.
    pub pickup: Option<PickupSpawn>,
}

/// What a behavior is given each time it runs. Read-only view of the caster.
pub struct AbilityContext<'a> {
    pub owner: Entity,
    /// Caster world position.
    pub origin: Vec2,
    /// Caster aim direction, normalized.
    pub facing: Vec2,
    /// Candidate targets (opposing faction) gathered by the execute system.
    pub targets: &'a [Target],
    /// Total simulated time elapsed since the app started (`Time::elapsed_secs`), Phase 9.3. Feeds
    /// `Orbiting`'s continuous rotation — deterministic under the sim's `ManualDuration` clock, so
    /// two identically-seeded runs compute the exact same hammer angle every frame.
    pub elapsed_secs: f32,
}

/// The base execution logic for one ability shape (melee cone, self nova, projectile, …).
/// Registered once in AbilityPlugin::build; referenced by BehaviorId string from RON.
pub trait AbilityBehavior: Send + Sync + 'static {
    /// Resolve which entities this cast hits (and any VFX / projectile). Pure — no world access.
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome;

    /// Direction-dependent shapes (cone, projectile) require a non-zero aim before firing;
    /// self-centred shapes (nova) do not. The execute system skips a needs_aim cast — without
    /// consuming its cooldown — while the caster has no aim.
    fn needs_aim(&self) -> bool {
        true
    }

    /// Whether a cast that resolves *no* hits and *no* projectile still spends its cooldown.
    /// Aimed/nova casts return `true` — a whiff into empty space still commits the swing (the
    /// prototype behavior, unchanged). `contact_melee` returns `false`, so an out-of-range enemy
    /// stays charged and strikes the instant it reaches the player (the old enemy_attack cadence).
    fn consumes_cooldown_on_whiff(&self) -> bool {
        true
    }
}

/// Resource: maps BehaviorId → boxed behavior. Populated at plugin build; read-only at runtime.
#[derive(Resource, Default)]
pub struct BehaviorRegistry {
    behaviors: HashMap<BehaviorId, Box<dyn AbilityBehavior>>,
}

impl BehaviorRegistry {
    pub fn register(&mut self, id: impl Into<BehaviorId>, behavior: impl AbilityBehavior) {
        self.behaviors.insert(id.into(), Box::new(behavior));
    }

    pub fn get(&self, id: &str) -> Option<&dyn AbilityBehavior> {
        self.behaviors.get(id).map(|b| b.as_ref())
    }
}

/// Picks the hit nearest to `origin` (ties → first in iteration order) for PrimaryHit scoping.
fn nearest(hits: &[HitTarget], origin: Vec2) -> Option<HitTarget> {
    hits.iter()
        .copied()
        .min_by(|a, b| {
            let da = a.pos.distance_squared(origin);
            let db = b.pos.distance_squared(origin);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
}

// ── Built-in behaviors ──────────────────────────────────────────────────────────────────

/// Melee cone (Death Strike, Scratch). Resolves every enemy within `range` and within
/// `half_angle` of the aim direction as a hit, and requests a cone VFX flash. Damage / leech /
/// status are the ability's `effects` — not decided here. Reproduces the prototype's
/// `player_arc_attack` geometry, now data-driven.
///
/// Params: "range", "half_angle" (+ whatever the ability's effects reference).
pub struct MeleeCone;

impl AbilityBehavior for MeleeCone {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let range = params.get("range");
        let half_angle = params.get("half_angle");
        let forward = ctx.facing;

        let mut hits = Vec::new();
        for target in ctx.targets {
            let to_target = target.pos - ctx.origin;
            let dist = to_target.length();
            if dist > range {
                continue;
            }
            // An enemy exactly at the origin has no direction; count it as inside the cone.
            let in_cone = dist < 1e-6 || forward.angle_to(to_target / dist).abs() <= half_angle;
            if !in_cone {
                continue;
            }
            hits.push(HitTarget { entity: target.entity, pos: target.pos });
        }

        let primary = nearest(&hits, ctx.origin);
        CastOutcome {
            origin: ctx.origin,
            primary,
            hits,
            vfx: Some(VfxShape::Cone { radius: range, half_angle, forward, lifetime: ATTACK_LIFETIME }),
            ..Default::default()
        }
    }
}

/// Self-centred nova (Blood Boil). Resolves every enemy within `radius` of the caster as a hit;
/// no aim required, so it auto-casts cleanly. Damage / leech / status are the ability's `effects`.
///
/// Params: "radius" (+ whatever the ability's effects reference).
pub struct SelfNova;

impl AbilityBehavior for SelfNova {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let radius = params.get("radius");
        let mut hits = Vec::new();
        for target in ctx.targets {
            if target.pos.distance(ctx.origin) <= radius {
                hits.push(HitTarget { entity: target.entity, pos: target.pos });
            }
        }
        let primary = nearest(&hits, ctx.origin);
        CastOutcome { origin: ctx.origin, hits, primary, ..Default::default() }
    }

    fn needs_aim(&self) -> bool {
        false
    }
}

/// Nearest-N melee (Heart Strike, Phase 9.2). Resolves up to `target_count` of the NEAREST enemies
/// within `range` as hits — distinct from `SelfNova`'s "everyone in radius" (unbounded count). No
/// aim required (self-centred, like SelfNova). Missing-health damage scaling and the D&D/execute
/// talents are the ability's innate/talent-gated hooks (ability/hooks.rs), not decided here.
///
/// Params: "range", "target_count" (+ whatever the ability's effects reference).
pub struct NearestMelee;

impl AbilityBehavior for NearestMelee {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let range = params.get("range");
        let target_count = params.get("target_count").max(0.0) as usize;

        let mut hits: Vec<HitTarget> = ctx
            .targets
            .iter()
            .filter(|t| t.pos.distance(ctx.origin) <= range)
            .map(|t| HitTarget { entity: t.entity, pos: t.pos })
            .collect();
        hits.sort_by(|a, b| {
            let da = a.pos.distance_squared(ctx.origin);
            let db = b.pos.distance_squared(ctx.origin);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(target_count);

        let primary = nearest(&hits, ctx.origin);
        CastOutcome { origin: ctx.origin, hits, primary, ..Default::default() }
    }

    fn needs_aim(&self) -> bool {
        false
    }
}

/// Enemy contact melee (grunt/runner/brute). Hits opposing-faction actors (the player) within
/// `range` of the caster — a proximity strike, no aim. Does **not** spend its cooldown on a whiff,
/// so a chasing enemy charges its swing while approaching and lands the first hit the instant it
/// reaches contact range, reproducing the prototype's `enemy_attack` cadence. Damage is the
/// ability's `effects` (Physical to the player), not decided here.
///
/// Params: "range" (+ whatever the ability's effects reference, e.g. "damage").
pub struct ContactMelee;

impl AbilityBehavior for ContactMelee {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let range = params.get("range");
        let mut hits = Vec::new();
        for target in ctx.targets {
            if target.pos.distance(ctx.origin) <= range {
                hits.push(HitTarget { entity: target.entity, pos: target.pos });
            }
        }
        let primary = nearest(&hits, ctx.origin);
        CastOutcome { origin: ctx.origin, hits, primary, ..Default::default() }
    }

    fn needs_aim(&self) -> bool {
        false
    }

    fn consumes_cooldown_on_whiff(&self) -> bool {
        false
    }
}

/// Travelling projectile (Fireblast, Frostbolt). Spawns a projectile heading along the aim
/// direction; it applies the ability's effects to the first enemy it collides with (and up to
/// `pierce` more). No instant hits — delivery is deferred to projectile/systems/collision.rs.
///
/// Params: "speed", "radius", "range" (→ lifetime = range/speed), "pierce".
pub struct ProjectileBehavior;

impl AbilityBehavior for ProjectileBehavior {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let speed = params.get("speed");
        let range = params.get("range");
        let radius = params.get("radius");
        let pierce = params.get("pierce").max(0.0) as u32;
        // Time to live so the shot travels ~range before expiring (fallback 2s if speed is 0).
        let lifetime = if speed > 1e-3 { range / speed } else { 2.0 };

        CastOutcome {
            origin: ctx.origin,
            projectile: Some(ProjectileSpawn {
                velocity: ctx.facing * speed,
                radius,
                pierce,
                lifetime,
            }),
            ..Default::default()
        }
    }
}

/// Dropped ground zone (D&D, Consecrated Ground, AMZ, Tree Conduit). Drops a persistent zone at the
/// caster's position; no aim required, so it auto-casts cleanly. What the zone *does* — feed the
/// `PlayerZonePresence` query, tick damage/regen to occupants, block projectiles — is decided by the
/// ability's `zone` spec + params and handled by the zone module, not here. New in Phase 6.
///
/// Params: "zone_radius", "zone_duration" (+ any occupant-effect params the zone reads).
pub struct DroppedZone;

impl AbilityBehavior for DroppedZone {
    fn resolve(&self, ctx: &AbilityContext, _params: &ResolvedParams) -> CastOutcome {
        CastOutcome {
            origin: ctx.origin,
            zone: Some(ZoneSpawn { center: ctx.origin }),
            ..Default::default()
        }
    }

    fn needs_aim(&self) -> bool {
        false
    }
}

/// Movement ability (Shift/Space — Mechanics' `InputSlot::Movement`; Phase 9.1, §8.1(3)/§8.5). No
/// targets, no damage: requests a short forced-movement impulse along the caster's facing. The
/// execute system turns the request into a `ForcedImpulse` on the caster.
///
/// Params: "speed", "duration".
pub struct Blink;

impl AbilityBehavior for Blink {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let speed = params.get("speed");
        let duration = params.get("duration");
        CastOutcome {
            origin: ctx.origin,
            forced_impulse: Some(ForcedImpulseSpawn { velocity: ctx.facing * speed, duration }),
            ..Default::default()
        }
    }
}

/// Summon (Phase 9.2 — Companion). No targets, no instant damage: signals "spawn a minion here."
/// The ability's `summon` spec (`AbilityDef.summon`) + resolved params supply everything else
/// (which ability the minion mimics, how long it lives); the execute system builds the minion
/// entity. No aim required — a passive, no-input ability (auto-cast).
pub struct Summon;

impl AbilityBehavior for Summon {
    fn resolve(&self, ctx: &AbilityContext, _params: &ResolvedParams) -> CastOutcome {
        CastOutcome {
            origin: ctx.origin,
            summon: Some(SummonSpawn),
            ..Default::default()
        }
    }

    fn needs_aim(&self) -> bool {
        false
    }
}

/// Grip (Phase 9.2 — Abomination Limb). Periodically pulls up to `target_count` of the nearest
/// enemies within `range` toward the caster (a `ForcedImpulse` per target, resolved by the execute
/// system). No aim required, no instant damage — grip is pure crowd control; the ability's
/// `effects` list stays empty. `ranged_only` (0.0/1.0, set by the epic talent via `Override(1.0)`)
/// filters to enemies whose `AiBehavior` is `RangedCaster` only.
///
/// Params: "range", "target_count", "grip_speed", "grip_duration", "ranged_only".
pub struct Grip;

impl AbilityBehavior for Grip {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let range = params.get("range");
        let target_count = params.get("target_count").max(0.0) as usize;
        let grip_speed = params.get("grip_speed");
        let grip_duration = params.get("grip_duration");
        let ranged_only = params.get("ranged_only") > 0.5;

        let mut candidates: Vec<&Target> = ctx
            .targets
            .iter()
            .filter(|t| t.pos.distance(ctx.origin) <= range)
            .filter(|t| !ranged_only || t.is_ranged)
            .collect();
        candidates.sort_by(|a, b| {
            let da = a.pos.distance_squared(ctx.origin);
            let db = b.pos.distance_squared(ctx.origin);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(target_count);

        let grip_targets = candidates
            .iter()
            .map(|t| GripSpawn {
                target: t.entity,
                velocity: (ctx.origin - t.pos).normalize_or_zero() * grip_speed,
                duration: grip_duration,
            })
            .collect();

        CastOutcome { origin: ctx.origin, grip_targets, ..Default::default() }
    }

    fn needs_aim(&self) -> bool {
        false
    }
}

/// Orbiting hazard (Phase 9.3 — Paladin's Spinning Hammer). Continuously re-cast on a short
/// "maintenance" cooldown (see spinning_hammer.ability.ron); each cast computes the CURRENT
/// position of up to `hammer_count` hammers orbiting the caster at `orbit_radius`, evenly spaced
/// and rotating at `angular_speed` rad/s, driven by `ctx.elapsed_secs` (not per-instance state, so
/// the behavior stays a stateless, pure function of ctx+params like every other one here). Any
/// target within `hit_radius` of a hammer's position this instant is a hit — a target swept by two
/// overlapping hammers in the same tick is only hit once. No aim required (self-centred).
/// Holy-mark double damage is a targeted special-case in execute.rs (a per-target conditional the
/// generic effects pipeline can't express) — see its own doc comment.
///
/// Params: "orbit_radius", "hit_radius", "angular_speed", "hammer_count".
pub struct Orbiting;

impl AbilityBehavior for Orbiting {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let orbit_radius = params.get("orbit_radius");
        let hit_radius = params.get("hit_radius");
        let angular_speed = params.get("angular_speed");
        let hammer_count = params.get("hammer_count").max(1.0) as usize;

        let mut seen = std::collections::HashSet::new();
        let mut hits = Vec::new();
        for i in 0..hammer_count {
            let angle = ctx.elapsed_secs * angular_speed
                + (i as f32) * (std::f32::consts::TAU / hammer_count as f32);
            let hammer_pos = ctx.origin + Vec2::new(angle.cos(), angle.sin()) * orbit_radius;
            for target in ctx.targets {
                if target.pos.distance(hammer_pos) <= hit_radius && seen.insert(target.entity) {
                    hits.push(HitTarget { entity: target.entity, pos: target.pos });
                }
            }
        }
        let primary = nearest(&hits, ctx.origin);
        CastOutcome { origin: ctx.origin, hits, primary, ..Default::default() }
    }

    fn needs_aim(&self) -> bool {
        false
    }
}

/// Single-target-plus-cleave (Phase 9.3 — Paladin's Hammer of Justice). Acquires ONE primary
/// target: the nearest enemy within `range` and `half_angle` of the caster's aim (the same
/// targeting shape `MeleeCone` uses for its whole hit set). Then hits every OTHER enemy within
/// `cleave_range` of the primary's position, within `cleave_half_angle` of the direction from the
/// caster THROUGH the primary (i.e. a cone opening "behind" the primary, away from the caster) —
/// the ability's `effects` (PrimaryHit full damage + SecondaryHits a `DamageFraction` of it) do the
/// rest. A whiff (no primary in range/arc) resolves no hits, no cleave.
///
/// Params: "range", "half_angle", "cleave_range", "cleave_half_angle" (+ whatever the ability's
/// effects reference, e.g. "damage", "cleave_fraction").
pub struct HammerCleave;

impl AbilityBehavior for HammerCleave {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let range = params.get("range");
        let half_angle = params.get("half_angle");
        let cleave_range = params.get("cleave_range");
        let cleave_half_angle = params.get("cleave_half_angle");
        let forward = ctx.facing;

        let mut in_arc: Vec<&Target> = ctx
            .targets
            .iter()
            .filter(|t| {
                let to = t.pos - ctx.origin;
                let dist = to.length();
                dist <= range && (dist < 1e-6 || forward.angle_to(to / dist).abs() <= half_angle)
            })
            .collect();
        in_arc.sort_by(|a, b| {
            let da = a.pos.distance_squared(ctx.origin);
            let db = b.pos.distance_squared(ctx.origin);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });

        let Some(primary_target) = in_arc.first() else {
            return CastOutcome {
                origin: ctx.origin,
                vfx: Some(VfxShape::Cone { radius: range, half_angle, forward, lifetime: ATTACK_LIFETIME }),
                ..Default::default()
            };
        };
        let primary_entity = primary_target.entity;
        let primary_pos = primary_target.pos;
        let cleave_dir = (primary_pos - ctx.origin).normalize_or_zero();

        let mut hits = vec![HitTarget { entity: primary_entity, pos: primary_pos }];
        for t in ctx.targets {
            if t.entity == primary_entity {
                continue;
            }
            let to = t.pos - primary_pos;
            let dist = to.length();
            let in_cleave = dist <= cleave_range
                && (dist < 1e-6 || cleave_dir.angle_to(to / dist).abs() <= cleave_half_angle);
            if in_cleave {
                hits.push(HitTarget { entity: t.entity, pos: t.pos });
            }
        }

        CastOutcome {
            origin: ctx.origin,
            primary: Some(HitTarget { entity: primary_entity, pos: primary_pos }),
            hits,
            vfx: Some(VfxShape::Cone { radius: range, half_angle, forward, lifetime: ATTACK_LIFETIME }),
            ..Default::default()
        }
    }
}

/// Channel-while-moving (Phase 9.3 — Paladin's Flash of Light; later Druid Heal / Mage Frost
/// Impale reuse it). No targets, no instant effects — the behavior only signals "start a
/// `cast_time`-second channel here." Everything else (the heal amount, overheal→shield, radiate,
/// the consecrated-ground epic) is resolved by `ability::systems::channel::tick_channels` when the
/// channel completes. No aim required — this is a self-cast.
///
/// Params: "cast_time".
pub struct ChannelWhileMoving;

impl AbilityBehavior for ChannelWhileMoving {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        CastOutcome {
            origin: ctx.origin,
            channel: Some(ChannelSpawn { cast_time: params.get("cast_time") }),
            ..Default::default()
        }
    }

    fn needs_aim(&self) -> bool {
        false
    }
}

/// Leap to a single target (Phase 9.4 — Druid's Ferocious Bite / Primal Pounce). Picks ONE target
/// within `leap_range`, requests a `ForcedImpulse` toward it (the visual/positional "jump" — a
/// short dash, not a guaranteed landing-on-top), and resolves it as the `primary` hit for the
/// ability's own `effects` (PrimaryHit damage, etc.) to apply. Two selection modes, chosen by the
/// numeric flag `select_highest_health` (the same "escape-hatch param flag" pattern as
/// `follow_caster`/`slow_active` elsewhere):
///   - `0.0` (Ferocious Bite: "jump to the closest target near your cursor") — nearest target
///     within `leap_range` AND within `half_angle` of the caster's aim. If the caster has no aim
///     (`facing` ~ zero — e.g. an AutoCast leap with this mode, which no shipped ability uses today)
///     the angle filter is skipped defensively, same as `MeleeCone`'s own degenerate-direction case.
///   - `1.0` (Primal Pounce: "automatically leap towards the highest-health enemy within a radius")
///     — the highest-`health` target within `leap_range`, ties broken by nearest distance then
///     iteration order (deterministic, no RunRng).
/// `needs_aim()` is `false` for BOTH modes — Primal Pounce is a self-centred AutoCast that must
/// fire with no player aim at all, so the aim gate can't live at the execute-system level here;
/// mode 0's own angle filter (above) is what actually makes it "prefer the cursor direction."
///
/// Params: "leap_range", "half_angle" (mode 0 only), "select_highest_health", "leap_speed",
/// "leap_duration" (+ whatever the ability's effects reference, e.g. "damage").
pub struct LeapToTarget;

impl AbilityBehavior for LeapToTarget {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let leap_range = params.get("leap_range");
        let half_angle = params.get("half_angle");
        let leap_speed = params.get("leap_speed");
        let leap_duration = params.get("leap_duration");
        let highest_health_mode = params.get("select_highest_health") > 0.5;
        let forward = ctx.facing;
        let has_aim = forward.length_squared() >= 1e-6;

        let in_range: Vec<&Target> = ctx
            .targets
            .iter()
            .filter(|t| t.pos.distance(ctx.origin) <= leap_range)
            .collect();

        let picked = if highest_health_mode {
            in_range.into_iter().min_by(|a, b| {
                // Highest health first; ties broken by nearest distance (deterministic).
                b.health
                    .partial_cmp(&a.health)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        let da = a.pos.distance_squared(ctx.origin);
                        let db = b.pos.distance_squared(ctx.origin);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
            })
        } else {
            in_range
                .into_iter()
                .filter(|t| {
                    if !has_aim {
                        return true;
                    }
                    let to = t.pos - ctx.origin;
                    let dist = to.length();
                    dist < 1e-6 || forward.angle_to(to / dist).abs() <= half_angle
                })
                .min_by(|a, b| {
                    let da = a.pos.distance_squared(ctx.origin);
                    let db = b.pos.distance_squared(ctx.origin);
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
        };

        let Some(target) = picked else {
            return CastOutcome { origin: ctx.origin, ..Default::default() };
        };

        let velocity = (target.pos - ctx.origin).normalize_or_zero() * leap_speed;
        CastOutcome {
            origin: ctx.origin,
            primary: Some(HitTarget { entity: target.entity, pos: target.pos }),
            hits: vec![HitTarget { entity: target.entity, pos: target.pos }],
            forced_impulse: Some(ForcedImpulseSpawn { velocity, duration: leap_duration }),
            ..Default::default()
        }
    }

    fn needs_aim(&self) -> bool {
        false
    }
}

/// Aimed burst at a distance (Phase 9.5 — Mage's Flamestrike). Unlike `SelfNova` (centred on the
/// caster's own origin), the blast centre is offset `cast_range` along the caster's aim — "cast a
/// fiery circle" at a chosen spot, not around yourself. Resolves every target within `zone_radius`
/// of that point. Needs aim (an un-aimed cast has nowhere to place the circle). "Increased damage
/// per enemy affected by blaze" is a targeted execute.rs special-case (the generic effects pipeline
/// applies one uniform amount per hit, not a per-cast count-scaled bonus).
///
/// Params: "cast_range", "zone_radius" (+ whatever the ability's effects reference).
pub struct TargetedBurst;

impl AbilityBehavior for TargetedBurst {
    fn resolve(&self, ctx: &AbilityContext, params: &ResolvedParams) -> CastOutcome {
        let cast_range = params.get("cast_range");
        let zone_radius = params.get("zone_radius");
        let center = ctx.origin + ctx.facing * cast_range;

        let hits: Vec<HitTarget> = ctx
            .targets
            .iter()
            .filter(|t| t.pos.distance(center) <= zone_radius)
            .map(|t| HitTarget { entity: t.entity, pos: t.pos })
            .collect();
        let primary = nearest(&hits, center);

        CastOutcome { origin: ctx.origin, hits, primary, ..Default::default() }
    }
}

/// Drop a collectible pickup at the caster's origin (Phase 9.4 — Druid's Bloom). No targets, no
/// aim — a passive, self-centred AutoCast like `Summon`/`DroppedZone`.
pub struct Bloom;

impl AbilityBehavior for Bloom {
    fn resolve(&self, ctx: &AbilityContext, _params: &ResolvedParams) -> CastOutcome {
        CastOutcome {
            origin: ctx.origin,
            pickup: Some(PickupSpawn),
            ..Default::default()
        }
    }

    fn needs_aim(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(pairs: &[(&str, f32)]) -> ResolvedParams {
        ResolvedParams(pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect())
    }

    #[test]
    fn melee_cone_resolves_only_enemies_in_range_and_arc() {
        let owner = Entity::from_raw(1);
        let in_cone = Entity::from_raw(2); // 30 ahead, dead centre
        let out_of_range = Entity::from_raw(3); // 100 to the side
        let outside_arc = Entity::from_raw(4); // ~53° off the aim, within range
        let targets = [
            Target { entity: in_cone, pos: Vec2::new(30.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: out_of_range, pos: Vec2::new(0.0, 100.0), is_ranged: false, health: 100.0 },
            Target { entity: outside_arc, pos: Vec2::new(30.0, 40.0), is_ranged: false, health: 100.0 },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[("range", 60.0), ("half_angle", 0.785)]); // ~45°

        let outcome = MeleeCone.resolve(&ctx, &p);

        let hit_entities: Vec<Entity> = outcome.hits.iter().map(|h| h.entity).collect();
        assert_eq!(hit_entities, vec![in_cone], "only the in-range, in-arc enemy is hit");
        assert_eq!(outcome.primary.map(|h| h.entity), Some(in_cone), "primary is the single hit");
        assert!(
            matches!(outcome.vfx, Some(VfxShape::Cone { .. })),
            "requests a cone VFX flash",
        );
    }

    #[test]
    fn melee_cone_primary_is_nearest_hit() {
        let owner = Entity::from_raw(1);
        let near = Entity::from_raw(2);
        let far = Entity::from_raw(3);
        // Both dead ahead and inside a wide cone; `far` is listed first to prove primary is
        // chosen by distance, not iteration order.
        let targets = [
            Target { entity: far, pos: Vec2::new(50.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: near, pos: Vec2::new(20.0, 0.0), is_ranged: false, health: 100.0 },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[("range", 60.0), ("half_angle", 0.785)]);

        let outcome = MeleeCone.resolve(&ctx, &p);
        assert_eq!(outcome.hits.len(), 2, "both enemies are in the cone");
        assert_eq!(outcome.primary.map(|h| h.entity), Some(near), "primary is the nearest hit");
    }

    #[test]
    fn nearest_melee_caps_to_target_count_nearest_within_range() {
        let owner = Entity::from_raw(1);
        let near = Entity::from_raw(2);
        let mid = Entity::from_raw(3);
        let far_in_range = Entity::from_raw(4);
        let out_of_range = Entity::from_raw(5);
        // Listed out of distance order to prove sorting, not iteration order, decides the cut.
        let targets = [
            Target { entity: far_in_range, pos: Vec2::new(40.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: out_of_range, pos: Vec2::new(90.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: near, pos: Vec2::new(10.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: mid, pos: Vec2::new(20.0, 0.0), is_ranged: false, health: 100.0 },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[("range", 60.0), ("target_count", 2.0)]);

        let outcome = NearestMelee.resolve(&ctx, &p);

        let hit_entities: Vec<Entity> = outcome.hits.iter().map(|h| h.entity).collect();
        assert_eq!(hit_entities, vec![near, mid], "only the 2 nearest in-range targets are hit");
        assert_eq!(outcome.primary.map(|h| h.entity), Some(near));
    }

    #[test]
    fn nearest_melee_needs_no_aim() {
        assert!(!NearestMelee.needs_aim());
    }

    #[test]
    fn grip_pulls_the_nearest_target_count_toward_the_caster() {
        let owner = Entity::from_raw(1);
        let near = Entity::from_raw(2);
        let far_in_range = Entity::from_raw(3);
        let out_of_range = Entity::from_raw(4);
        let targets = [
            Target { entity: far_in_range, pos: Vec2::new(0.0, 100.0), is_ranged: false, health: 100.0 },
            Target { entity: out_of_range, pos: Vec2::new(0.0, 300.0), is_ranged: false, health: 100.0 },
            Target { entity: near, pos: Vec2::new(50.0, 0.0), is_ranged: false, health: 100.0 },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[("range", 150.0), ("target_count", 1.0), ("grip_speed", 100.0), ("grip_duration", 0.5), ("ranged_only", 0.0)]);

        let outcome = Grip.resolve(&ctx, &p);

        assert_eq!(outcome.grip_targets.len(), 1, "only target_count=1 nearest in-range target gripped");
        let grip = outcome.grip_targets[0];
        assert_eq!(grip.target, near);
        assert_eq!(grip.velocity, Vec2::new(-100.0, 0.0), "pulled toward the caster's origin");
        assert_eq!(grip.duration, 0.5);
        assert!(outcome.hits.is_empty(), "grip deals no instant damage");
    }

    #[test]
    fn grip_ranged_only_filters_out_melee_targets() {
        let owner = Entity::from_raw(1);
        let melee = Entity::from_raw(2);
        let ranged = Entity::from_raw(3);
        let targets = [
            Target { entity: melee, pos: Vec2::new(10.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: ranged, pos: Vec2::new(20.0, 0.0), is_ranged: true, health: 100.0 },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[("range", 100.0), ("target_count", 5.0), ("grip_speed", 100.0), ("grip_duration", 0.5), ("ranged_only", 1.0)]);

        let outcome = Grip.resolve(&ctx, &p);

        assert_eq!(outcome.grip_targets.len(), 1);
        assert_eq!(outcome.grip_targets[0].target, ranged, "ranged_only skips the melee target even though it's nearer");
    }

    #[test]
    fn blink_requests_a_forced_impulse_along_facing_with_no_targets() {
        let owner = Entity::from_raw(1);
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::Y, targets: &[], elapsed_secs: 0.0 };
        let p = params(&[("speed", 500.0), ("duration", 0.15)]);

        let outcome = Blink.resolve(&ctx, &p);

        assert!(outcome.hits.is_empty(), "blink hits nothing");
        assert!(outcome.projectile.is_none());
        assert!(outcome.zone.is_none());
        let impulse = outcome.forced_impulse.expect("blink requests a forced impulse");
        assert_eq!(impulse.velocity, Vec2::Y * 500.0, "impulse velocity follows facing * speed");
        assert_eq!(impulse.duration, 0.15);
    }

    #[test]
    fn orbiting_hits_a_target_only_while_the_rotating_hammer_is_near_it() {
        let owner = Entity::from_raw(1);
        // A single hammer orbits at radius 50, angular_speed pi/s -> at t=0 it's at (50, 0);
        // at t=1 (pi rad later, half a turn) it's at (-50, 0).
        let target = Entity::from_raw(2);
        let targets = [Target { entity: target, pos: Vec2::new(50.0, 0.0), is_ranged: false, health: 100.0 }];
        let p = params(&[
            ("orbit_radius", 50.0),
            ("hit_radius", 10.0),
            ("angular_speed", std::f32::consts::PI),
            ("hammer_count", 1.0),
        ]);

        let ctx0 = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let outcome0 = Orbiting.resolve(&ctx0, &p);
        assert_eq!(outcome0.hits.len(), 1, "hammer starts at (50,0), right on the target");

        let ctx_half = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 1.0 };
        let outcome_half = Orbiting.resolve(&ctx_half, &p);
        assert!(outcome_half.hits.is_empty(), "half a turn later the hammer is at (-50,0), far from the target");
    }

    #[test]
    fn orbiting_two_hammers_are_evenly_spaced_and_each_hits_independently() {
        let owner = Entity::from_raw(1);
        let near_start = Entity::from_raw(2); // at (50, 0) -> hammer 0's t=0 position
        let near_opposite = Entity::from_raw(3); // at (-50, 0) -> hammer 1's t=0 position (offset by pi)
        let targets = [
            Target { entity: near_start, pos: Vec2::new(50.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: near_opposite, pos: Vec2::new(-50.0, 0.0), is_ranged: false, health: 100.0 },
        ];
        let p = params(&[
            ("orbit_radius", 50.0),
            ("hit_radius", 10.0),
            ("angular_speed", 0.0), // frozen in place so spacing alone is exercised
            ("hammer_count", 2.0),
        ]);
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let outcome = Orbiting.resolve(&ctx, &p);
        let hit_entities: std::collections::HashSet<Entity> = outcome.hits.iter().map(|h| h.entity).collect();
        assert_eq!(hit_entities.len(), 2, "both hammers land on their own evenly-spaced target");
    }

    #[test]
    fn hammer_cleave_hits_the_nearest_in_arc_primary_plus_a_cone_behind_it() {
        let owner = Entity::from_raw(1);
        let primary = Entity::from_raw(2); // dead ahead, primary acquisition
        let behind_primary = Entity::from_raw(3); // further along the same ray -> in the cleave cone
        let off_to_the_side = Entity::from_raw(4); // near the primary but off-axis -> outside the cleave cone
        let targets = [
            Target { entity: primary, pos: Vec2::new(30.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: behind_primary, pos: Vec2::new(60.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: off_to_the_side, pos: Vec2::new(30.0, 40.0), is_ranged: false, health: 100.0 },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[
            ("range", 50.0),
            ("half_angle", 0.2),
            ("cleave_range", 40.0),
            ("cleave_half_angle", 0.2),
        ]);

        let outcome = HammerCleave.resolve(&ctx, &p);

        assert_eq!(outcome.primary.map(|h| h.entity), Some(primary));
        let hit_entities: Vec<Entity> = outcome.hits.iter().map(|h| h.entity).collect();
        assert!(hit_entities.contains(&primary));
        assert!(hit_entities.contains(&behind_primary), "in the cleave cone behind the primary");
        assert!(!hit_entities.contains(&off_to_the_side), "outside the cleave cone's narrow arc");
    }

    #[test]
    fn hammer_cleave_whiffs_cleanly_with_no_primary_in_arc() {
        let owner = Entity::from_raw(1);
        let out_of_range = Entity::from_raw(2);
        let targets = [Target { entity: out_of_range, pos: Vec2::new(200.0, 0.0), is_ranged: false, health: 100.0 }];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[("range", 50.0), ("half_angle", 0.2), ("cleave_range", 40.0), ("cleave_half_angle", 0.2)]);

        let outcome = HammerCleave.resolve(&ctx, &p);
        assert!(outcome.hits.is_empty());
        assert!(outcome.primary.is_none());
    }

    #[test]
    fn channel_while_moving_requests_a_channel_with_no_instant_hits() {
        let owner = Entity::from_raw(1);
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::ZERO, targets: &[], elapsed_secs: 0.0 };
        let p = params(&[("cast_time", 1.5)]);

        let outcome = ChannelWhileMoving.resolve(&ctx, &p);

        assert!(outcome.hits.is_empty());
        let channel = outcome.channel.expect("requests a channel");
        assert_eq!(channel.cast_time, 1.5);
    }

    #[test]
    fn channel_while_moving_needs_no_aim() {
        assert!(!ChannelWhileMoving.needs_aim());
    }

    #[test]
    fn leap_to_target_cursor_mode_picks_the_nearest_in_arc() {
        let owner = Entity::from_raw(1);
        let near = Entity::from_raw(2);
        let far = Entity::from_raw(3);
        let out_of_arc = Entity::from_raw(4);
        let targets = [
            Target { entity: far, pos: Vec2::new(80.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: near, pos: Vec2::new(30.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: out_of_arc, pos: Vec2::new(30.0, 40.0), is_ranged: false, health: 100.0 },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[
            ("leap_range", 120.0),
            ("half_angle", 0.3),
            ("select_highest_health", 0.0),
            ("leap_speed", 400.0),
            ("leap_duration", 0.2),
        ]);

        let outcome = LeapToTarget.resolve(&ctx, &p);

        assert_eq!(outcome.primary.map(|h| h.entity), Some(near));
        let impulse = outcome.forced_impulse.expect("leap requests a forced impulse");
        assert_eq!(impulse.velocity, Vec2::X * 400.0, "leaps straight toward the picked target");
        assert_eq!(impulse.duration, 0.2);
    }

    #[test]
    fn leap_to_target_highest_health_mode_ignores_arc_and_distance() {
        let owner = Entity::from_raw(1);
        let weak_and_near = Entity::from_raw(2);
        let tanky_and_far = Entity::from_raw(3);
        let targets = [
            Target { entity: weak_and_near, pos: Vec2::new(10.0, 0.0), is_ranged: false, health: 5.0 },
            Target { entity: tanky_and_far, pos: Vec2::new(-90.0, 0.0), is_ranged: false, health: 500.0 },
        ];
        // Facing perpendicular to both — proves mode 1 doesn't filter by aim at all.
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::Y, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[
            ("leap_range", 120.0),
            ("half_angle", 0.1),
            ("select_highest_health", 1.0),
            ("leap_speed", 300.0),
            ("leap_duration", 0.2),
        ]);

        let outcome = LeapToTarget.resolve(&ctx, &p);
        assert_eq!(outcome.primary.map(|h| h.entity), Some(tanky_and_far), "picks the highest health, not nearest/in-arc");
    }

    #[test]
    fn leap_to_target_whiffs_cleanly_with_nothing_in_range() {
        let owner = Entity::from_raw(1);
        let out_of_range = Entity::from_raw(2);
        let targets = [Target { entity: out_of_range, pos: Vec2::new(500.0, 0.0), is_ranged: false, health: 10.0 }];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[("leap_range", 50.0), ("half_angle", 0.3), ("select_highest_health", 0.0), ("leap_speed", 300.0), ("leap_duration", 0.2)]);

        let outcome = LeapToTarget.resolve(&ctx, &p);
        assert!(outcome.primary.is_none());
        assert!(outcome.forced_impulse.is_none());
    }

    #[test]
    fn leap_to_target_needs_no_aim() {
        assert!(!LeapToTarget.needs_aim());
    }

    #[test]
    fn targeted_burst_hits_only_within_zone_radius_of_the_offset_center() {
        let owner = Entity::from_raw(1);
        let inside = Entity::from_raw(2); // near the offset center (200, 0)
        let at_caster = Entity::from_raw(3); // near the CASTER, not the offset center
        let targets = [
            Target { entity: inside, pos: Vec2::new(210.0, 0.0), is_ranged: false, health: 100.0 },
            Target { entity: at_caster, pos: Vec2::new(10.0, 0.0), is_ranged: false, health: 100.0 },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[("cast_range", 200.0), ("zone_radius", 30.0)]);

        let outcome = TargetedBurst.resolve(&ctx, &p);

        let hit_entities: Vec<Entity> = outcome.hits.iter().map(|h| h.entity).collect();
        assert_eq!(hit_entities, vec![inside], "only the target near the offset blast center is hit");
        assert_eq!(outcome.primary.map(|h| h.entity), Some(inside));
    }

    #[test]
    fn targeted_burst_whiffs_cleanly_with_nothing_near_the_center() {
        let owner = Entity::from_raw(1);
        let far = Entity::from_raw(2);
        let targets = [Target { entity: far, pos: Vec2::new(500.0, 500.0), is_ranged: false, health: 100.0 }];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets, elapsed_secs: 0.0 };
        let p = params(&[("cast_range", 200.0), ("zone_radius", 30.0)]);

        let outcome = TargetedBurst.resolve(&ctx, &p);
        assert!(outcome.hits.is_empty());
        assert!(outcome.primary.is_none());
    }

    #[test]
    fn bloom_requests_a_pickup_with_no_targets_or_aim() {
        let owner = Entity::from_raw(1);
        let ctx = AbilityContext { owner, origin: Vec2::new(5.0, 7.0), facing: Vec2::ZERO, targets: &[], elapsed_secs: 0.0 };
        let outcome = Bloom.resolve(&ctx, &params(&[]));
        assert!(outcome.pickup.is_some());
        assert_eq!(outcome.origin, Vec2::new(5.0, 7.0));
        assert!(!Bloom.needs_aim());
    }
}
