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
//   "dropped_zone"      — D&D / Consecrated Ground trail (Phase 6)
//   "periodic_self_zone" — self-centred pulsing zone (Phase 6)
//   "orbiting"          — Spinning Hammer (later)
//   "leap_to_target"    — Ferocious Bite (later)
//   "channel_while_moving" — Heal / Flash of Light / Frost Impale (later)
//   "summon"            — Companion (later)

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
            projectile: None,
            zone: None,
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
        CastOutcome { origin: ctx.origin, hits, primary, vfx: None, projectile: None, zone: None }
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
        CastOutcome { origin: ctx.origin, hits, primary, vfx: None, projectile: None, zone: None }
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
            hits: Vec::new(),
            primary: None,
            vfx: None,
            projectile: Some(ProjectileSpawn {
                velocity: ctx.facing * speed,
                radius,
                pierce,
                lifetime,
            }),
            zone: None,
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
            Target { entity: in_cone, pos: Vec2::new(30.0, 0.0) },
            Target { entity: out_of_range, pos: Vec2::new(0.0, 100.0) },
            Target { entity: outside_arc, pos: Vec2::new(30.0, 40.0) },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets };
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
            Target { entity: far, pos: Vec2::new(50.0, 0.0) },
            Target { entity: near, pos: Vec2::new(20.0, 0.0) },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, targets: &targets };
        let p = params(&[("range", 60.0), ("half_angle", 0.785)]);

        let outcome = MeleeCone.resolve(&ctx, &p);
        assert_eq!(outcome.hits.len(), 2, "both enemies are in the cone");
        assert_eq!(outcome.primary.map(|h| h.entity), Some(near), "primary is the nearest hit");
    }
}
