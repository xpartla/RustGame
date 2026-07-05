// Shared effect application: the single place an ability's declarative effects turn into
// DamageEvent / HealEvent / ApplyStatusEvent. Used by BOTH the instant cast path
// (systems/execute.rs) and the deferred projectile-impact path (projectile/systems/collision.rs),
// so a fire projectile and a melee cone apply damage/status through identical logic.
//
// EffectSpec references param *keys*; `resolve_effects` bakes them to numbers (ResolvedEffect) at
// cast time. A projectile carries the baked effects so it needs no param/asset access on impact —
// talent changes mid-flight don't retroactively alter an in-flight shot (intended).

use bevy::prelude::*;
use crate::ability::assets::{EffectSpec, EffectTarget};
use crate::ability::behavior::{HitTarget, ResolvedParams};
use crate::core::events::{DamageEvent, DamageTag, HealEvent};
use crate::status::components::ApplyStatusEvent;

/// An EffectSpec with its param references resolved to concrete numbers.
#[derive(Debug, Clone)]
pub enum ResolvedEffect {
    Damage { amount: f32, tags: Vec<DamageTag>, target: EffectTarget },
    Heal { amount: f32, target: EffectTarget },
    Leech { percent: f32 },
    ApplyStatus { status: String, stacks: u8, target: EffectTarget },
}

/// Bakes an ability's effect list against its resolved params.
pub fn resolve_effects(effects: &[EffectSpec], params: &ResolvedParams) -> Vec<ResolvedEffect> {
    effects
        .iter()
        .map(|e| match e {
            EffectSpec::Damage { amount, tags, target } => ResolvedEffect::Damage {
                amount: params.get(amount),
                tags: tags.clone(),
                target: *target,
            },
            EffectSpec::Heal { amount, target } => ResolvedEffect::Heal {
                amount: params.get(amount),
                target: *target,
            },
            EffectSpec::Leech { percent } => ResolvedEffect::Leech { percent: params.get(percent) },
            EffectSpec::ApplyStatus { status, stacks, target } => ResolvedEffect::ApplyStatus {
                status: status.clone(),
                stacks: *stacks,
                target: *target,
            },
        })
        .collect()
}

/// Applies baked effects against a hit set. Two passes so Leech can use the total damage dealt
/// regardless of effect order in the RON (matches the Phase-1/2 Death Strike ordering exactly).
pub fn apply_resolved_effects(
    damage_events: &mut EventWriter<DamageEvent>,
    heal_events: &mut EventWriter<HealEvent>,
    status_events: &mut EventWriter<ApplyStatusEvent>,
    caster: Entity,
    hits: &[HitTarget],
    primary: Option<HitTarget>,
    effects: &[ResolvedEffect],
) {
    let mut total_damage = 0.0;
    for effect in effects {
        match effect {
            ResolvedEffect::Damage { amount, tags, target } => {
                for t in resolve_targets(*target, hits, primary, caster) {
                    damage_events.write(DamageEvent { target: t, amount: *amount, source: caster, tags: tags.clone() });
                    total_damage += *amount;
                }
            }
            ResolvedEffect::Heal { amount, target } => {
                for t in resolve_targets(*target, hits, primary, caster) {
                    heal_events.write(HealEvent { target: t, amount: *amount });
                }
            }
            ResolvedEffect::ApplyStatus { status, stacks, target } => {
                for t in resolve_targets(*target, hits, primary, caster) {
                    status_events.write(ApplyStatusEvent {
                        target: t,
                        source: caster,
                        effect_id: status.clone(),
                        stacks: *stacks,
                    });
                }
            }
            ResolvedEffect::Leech { .. } => {} // second pass — needs total_damage
        }
    }
    for effect in effects {
        if let ResolvedEffect::Leech { percent } = effect {
            let heal = total_damage * percent / 100.0;
            if heal > 0.0 {
                heal_events.write(HealEvent { target: caster, amount: heal });
            }
        }
    }
}

/// Maps an EffectTarget to concrete entities from a hit set.
fn resolve_targets(target: EffectTarget, hits: &[HitTarget], primary: Option<HitTarget>, caster: Entity) -> Vec<Entity> {
    match target {
        EffectTarget::AllHits => hits.iter().map(|h| h.entity).collect(),
        EffectTarget::PrimaryHit => primary.iter().map(|h| h.entity).collect(),
        EffectTarget::Caster => vec![caster],
    }
}
