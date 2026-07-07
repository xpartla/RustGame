// Shared effect application: the single place an ability's declarative effects turn into
// DamageEvent / HealEvent / ApplyStatusEvent. Used by BOTH the instant cast path
// (systems/execute.rs) and the deferred projectile-impact path (projectile/systems/collision.rs),
// so a fire projectile and a melee cone apply damage/status through identical logic.
//
// EffectSpec references param *keys*; `resolve_effects` bakes them to numbers (ResolvedEffect) at
// cast time. A projectile carries the baked effects so it needs no param/asset access on impact —
// talent changes mid-flight don't retroactively alter an in-flight shot (intended).
//
// Crit (Phase 9.1, §8.1(4)): `Damage` also bakes `crit_chance`/`crit_mult` from the universal stat
// baseline (talent/modifier.rs). The roll itself happens here, per target, at application time —
// `roll_crit` only draws from `RunRng` when `crit_chance > 0.0`, so an ability with no crit talent
// (every shipped ability today) never perturbs the RNG stream: byte-identical golden master.

use bevy::prelude::*;
use rand::Rng;
use crate::ability::assets::{EffectSpec, EffectTarget};
use crate::ability::behavior::{HitTarget, ResolvedParams};
use crate::core::events::{DamageEvent, DamageTag, HealEvent};
use crate::run::rng::RunRng;
use crate::status::components::ApplyStatusEvent;

/// An EffectSpec with its param references resolved to concrete numbers.
#[derive(Debug, Clone)]
pub enum ResolvedEffect {
    Damage { amount: f32, tags: Vec<DamageTag>, target: EffectTarget, crit_chance: f32, crit_mult: f32 },
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
                crit_chance: params.get("crit_chance"),
                crit_mult: params.get("crit_mult"),
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

/// Rolls whether a single hit crits. `crit_chance` is a percentage (0..100, matching the
/// `leech_percent` convention). Short-circuits without touching `rng` when `crit_chance <= 0.0` —
/// the guarantee that keeps every crit-less cast (all shipped content today) from perturbing the
/// deterministic `RunRng` stream the golden master pins.
fn roll_crit(rng: &mut RunRng, crit_chance: f32) -> bool {
    crit_chance > 0.0 && rng.rng().gen_range(0.0..100.0) < crit_chance
}

/// Applies baked effects against a hit set. Two passes so Leech can use the total damage dealt
/// regardless of effect order in the RON (matches the Phase-1/2 Death Strike ordering exactly).
/// Each `Damage` hit independently rolls its own crit (`roll_crit`) — melee cone / self-nova casts
/// that hit several enemies don't all crit together.
pub fn apply_resolved_effects(
    damage_events: &mut EventWriter<DamageEvent>,
    heal_events: &mut EventWriter<HealEvent>,
    status_events: &mut EventWriter<ApplyStatusEvent>,
    rng: &mut RunRng,
    caster: Entity,
    hits: &[HitTarget],
    primary: Option<HitTarget>,
    effects: &[ResolvedEffect],
) {
    let mut total_damage = 0.0;
    for effect in effects {
        match effect {
            ResolvedEffect::Damage { amount, tags, target, crit_chance, crit_mult } => {
                for t in resolve_targets(*target, hits, primary, caster) {
                    let dealt = if roll_crit(rng, *crit_chance) { amount * crit_mult } else { *amount };
                    damage_events.write(DamageEvent { target: t, amount: dealt, source: caster, tags: tags.clone() });
                    total_damage += dealt;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roll_crit_never_draws_the_rng_when_chance_is_zero_or_negative() {
        // The byte-identical-golden-master guarantee: every ability without a crit talent resolves
        // crit_chance to 0.0 (talent/modifier.rs's universal baseline), so this must short-circuit.
        let mut rng = RunRng::from_seed(1);
        assert!(!roll_crit(&mut rng, 0.0));
        assert!(!roll_crit(&mut rng, -5.0));
    }

    #[test]
    fn roll_crit_always_succeeds_at_100_percent() {
        // gen_range(0.0..100.0) never reaches 100.0, so a 100% chance is deterministically true —
        // no need to hunt for a "lucky" seed to make this test non-flaky.
        let mut rng = RunRng::from_seed(1);
        for _ in 0..50 {
            assert!(roll_crit(&mut rng, 100.0));
        }
    }
}
