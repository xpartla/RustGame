use bevy::prelude::{Commands, EventReader, Query};
use crate::core::components::{Absorb, DamageDealtModifier, DamageTakenModifier, Health, Invulnerable, LastHitBy};
use crate::core::events::DamageEvent;

/// Drains up to `incoming` from `*absorb`, returning the leftover that spills through to `Health`.
/// Pure math core (Phase 9.1, §8.1(5)) — unit-tested directly; `apply_damage` wraps it with the
/// ECS query/removal. `incoming` is assumed non-negative (as every `DamageEvent.amount` is).
pub fn drain_absorb(absorb: &mut f32, incoming: f32) -> f32 {
    let drained = incoming.min(*absorb);
    *absorb -= drained;
    incoming - drained
}

/// The single point that mutates `Health`. Drains `DamageEvent`s and subtracts from the target's
/// health, scaled by the source's `DamageDealtModifier` (enemy scaling; 1.0 when absent) and the
/// target's `DamageTakenModifier` (frostbite +10%, etc.; 1.0 when absent). An `Invulnerable` target
/// (Phase 9.2 — Purgatory) discards the hit entirely, before the `Absorb` shield even drains. An
/// `Absorb` shield (Phase 9.1) drains BEFORE the health write, between the modifier scaling and the
/// subtraction; a hit larger than the pool spills the remainder to `Health`, and an emptied shield
/// is removed. Also records the dealer in the target's `LastHitBy` (if it tracks one) for kill-
/// credit. Death is handled separately (per-entity death systems read `Health`).
pub fn apply_damage(
    mut commands: Commands,
    mut events: EventReader<DamageEvent>,
    dealers: Query<&DamageDealtModifier>,
    mut targets: Query<(
        &mut Health,
        Option<&mut LastHitBy>,
        Option<&DamageTakenModifier>,
        Option<&mut Absorb>,
        Option<&Invulnerable>,
    )>,
) {
    for event in events.read() {
        let dealt_mult = dealers.get(event.source).map(|m| m.0).unwrap_or(1.0);
        if let Ok((mut health, last_hit_by, taken_mod, absorb, invulnerable)) = targets.get_mut(event.target) {
            if invulnerable.is_some() {
                continue;
            }
            let taken_mult = taken_mod.map(|m| m.0).unwrap_or(1.0);
            let mut incoming = event.amount * dealt_mult * taken_mult;
            if let Some(mut shield) = absorb {
                incoming = drain_absorb(&mut shield.amount, incoming);
                if shield.amount <= 0.0 {
                    commands.entity(event.target).remove::<Absorb>();
                }
            }
            health.current -= incoming;
            if let Some(mut last) = last_hit_by {
                last.0 = event.source;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_absorb_consumes_up_to_the_pool_and_returns_the_spill() {
        let mut pool = 10.0;
        // A hit smaller than the pool is fully absorbed; nothing spills.
        assert_eq!(drain_absorb(&mut pool, 6.0), 0.0);
        assert_eq!(pool, 4.0);
        // A hit larger than the remaining pool drains it and spills the remainder.
        assert_eq!(drain_absorb(&mut pool, 10.0), 6.0);
        assert_eq!(pool, 0.0);
    }

    #[test]
    fn drain_absorb_on_an_empty_pool_spills_everything() {
        let mut pool = 0.0;
        assert_eq!(drain_absorb(&mut pool, 5.0), 5.0);
        assert_eq!(pool, 0.0);
    }
}
