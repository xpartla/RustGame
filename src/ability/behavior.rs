// BehaviorRegistry — the open extension point of the ability system, plus the built-in
// behavior implementations. (The talent HookRegistry lands with the talent system in Phase 2.)
//
// Adding a new ability shape:
//   1. Implement AbilityBehavior for a unit struct.
//   2. Call registry.register("my_behavior", MyBehavior) in AbilityPlugin::build.
//   3. Set `behavior: "my_behavior"` in the ability's RON file.
//   No other code changes.
//
// Execution model (deliberately &mut World-free):
//   A behavior reads an AbilityContext (owner identity, position, aim, candidate targets) and
//   pushes AbilityEffects into a buffer. The execute system (systems/execute.rs) is the only
//   place that touches Commands / EventWriter — it drains the buffer and applies each effect.
//   This keeps behaviors pure and trivially testable, and matches architecture-plan §3.3.
//
// Behaviors still pending (registered in their phase; until then execute_ready_abilities skips
// any ability whose behavior id is not in the registry):
//   "projectile"        — travelling projectile (needs projectile movement + collision)
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
use crate::core::events::DamageTag;

/// Resolved numeric parameters after the talent modifier stack has been applied.
/// Produced by resolve_params() in ability/systems/resolve_params.rs. In Phase 1 (no talents
/// yet) this is just the ability's base params.
#[derive(Debug, Clone)]
pub struct ResolvedParams(pub HashMap<StatId, f32>);

impl ResolvedParams {
    /// Returns the param value, or 0.0 if the stat is not present.
    pub fn get(&self, stat: &str) -> f32 {
        *self.0.get(stat).unwrap_or(&0.0)
    }
}

/// A candidate target the execute system gathers up front and hands to the behavior.
/// Phase 1: every `Enemy`. Later this becomes faction-aware.
#[derive(Debug, Clone, Copy)]
pub struct EnemyTarget {
    pub entity: Entity,
    pub pos: Vec2,
}

/// A deferred side effect a behavior requests. The execute system applies these after the
/// behavior returns, so behaviors never touch Commands / EventWriter directly.
#[derive(Debug)]
pub enum AbilityEffect {
    /// Deal damage to `target`. `source` is filled in by the execute system (the owner).
    Damage {
        target: Entity,
        amount: f32,
        tags: Vec<DamageTag>,
    },
    /// Restore health to `target` (used for leech / self-heal abilities).
    Heal { target: Entity, amount: f32 },
    /// Spawn a transient cone VFX flash (reuses the projectile hitbox-gizmo entities).
    ConeVfx {
        origin: Vec2,
        radius: f32,
        half_angle: f32,
        forward: Vec2,
        lifetime: f32,
    },
}

/// What a behavior/hook is given each time it runs. Read-only view of the caster.
pub struct AbilityContext<'a> {
    pub owner: Entity,
    /// Caster world position.
    pub origin: Vec2,
    /// Caster aim direction, normalized.
    pub facing: Vec2,
    /// Candidate targets gathered by the execute system.
    pub enemies: &'a [EnemyTarget],
}

/// The base execution logic for one ability shape (melee cone, projectile, zone drop, …).
/// Registered once in AbilityPlugin::build; referenced by BehaviorId string from RON.
pub trait AbilityBehavior: Send + Sync + 'static {
    fn execute(&self, ctx: &AbilityContext, params: &ResolvedParams, effects: &mut Vec<AbilityEffect>);
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

// ── Built-in behaviors ──────────────────────────────────────────────────────────────────

/// Melee cone (Death Strike). Hits every enemy within `range` and within `half_angle` of the
/// aim direction, applies `leech_percent` of the damage dealt back as self-heal, and requests
/// a cone VFX flash. Reproduces the prototype's `player_arc_attack`, now data-driven.
///
/// Params: "damage", "range", "half_angle", "cooldown", "leech_percent".
pub struct MeleeCone;

impl AbilityBehavior for MeleeCone {
    fn execute(&self, ctx: &AbilityContext, params: &ResolvedParams, effects: &mut Vec<AbilityEffect>) {
        let damage = params.get("damage");
        let range = params.get("range");
        let half_angle = params.get("half_angle");
        let leech_percent = params.get("leech_percent");

        let forward = ctx.facing;
        let mut leech_total = 0.0;

        for target in ctx.enemies {
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

            effects.push(AbilityEffect::Damage {
                target: target.entity,
                amount: damage,
                tags: vec![DamageTag::Physical],
            });
            leech_total += damage * leech_percent / 100.0;
        }

        if leech_total > 0.0 {
            effects.push(AbilityEffect::Heal {
                target: ctx.owner,
                amount: leech_total,
            });
        }

        effects.push(AbilityEffect::ConeVfx {
            origin: ctx.origin,
            radius: range,
            half_angle,
            forward,
            lifetime: ATTACK_LIFETIME,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(pairs: &[(&str, f32)]) -> ResolvedParams {
        ResolvedParams(pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect())
    }

    #[test]
    fn melee_cone_hits_only_enemies_in_range_and_arc() {
        let owner = Entity::from_raw(1);
        let in_cone = Entity::from_raw(2);   // 30 ahead, dead centre
        let out_of_range = Entity::from_raw(3); // 100 to the side
        let outside_arc = Entity::from_raw(4);  // ~53° off the aim, within range
        let enemies = [
            EnemyTarget { entity: in_cone, pos: Vec2::new(30.0, 0.0) },
            EnemyTarget { entity: out_of_range, pos: Vec2::new(0.0, 100.0) },
            EnemyTarget { entity: outside_arc, pos: Vec2::new(30.0, 40.0) },
        ];
        let ctx = AbilityContext { owner, origin: Vec2::ZERO, facing: Vec2::X, enemies: &enemies };
        let p = params(&[
            ("damage", 10.0),
            ("range", 60.0),
            ("half_angle", 0.785), // ~45°
            ("leech_percent", 5.0),
        ]);

        let mut effects = Vec::new();
        MeleeCone.execute(&ctx, &p, &mut effects);

        let damaged: Vec<Entity> = effects
            .iter()
            .filter_map(|e| match e {
                AbilityEffect::Damage { target, amount, .. } => {
                    assert_eq!(*amount, 10.0);
                    Some(*target)
                }
                _ => None,
            })
            .collect();
        assert_eq!(damaged, vec![in_cone], "only the in-range, in-arc enemy is hit");

        let heal: f32 = effects
            .iter()
            .filter_map(|e| match e {
                AbilityEffect::Heal { target, amount } => {
                    assert_eq!(*target, owner);
                    Some(*amount)
                }
                _ => None,
            })
            .sum();
        assert!((heal - 0.5).abs() < 1e-6, "leech = 10 dmg * 5% = 0.5");

        assert!(
            effects.iter().any(|e| matches!(e, AbilityEffect::ConeVfx { .. })),
            "spawns a cone VFX flash",
        );
    }

    #[test]
    fn melee_cone_no_leech_no_heal_effect() {
        let ctx = AbilityContext {
            owner: Entity::from_raw(1),
            origin: Vec2::ZERO,
            facing: Vec2::X,
            enemies: &[EnemyTarget { entity: Entity::from_raw(2), pos: Vec2::new(10.0, 0.0) }],
        };
        let p = params(&[("damage", 10.0), ("range", 60.0), ("half_angle", 0.785), ("leech_percent", 0.0)]);

        let mut effects = Vec::new();
        MeleeCone.execute(&ctx, &p, &mut effects);

        assert!(!effects.iter().any(|e| matches!(e, AbilityEffect::Heal { .. })));
        assert_eq!(effects.iter().filter(|e| matches!(e, AbilityEffect::Damage { .. })).count(), 1);
    }
}
