use bevy::prelude::{EventReader, Query};
use crate::core::components::{DamageTakenModifier, Health, LastHitBy};
use crate::core::events::DamageEvent;

/// The single point that mutates `Health`. Drains `DamageEvent`s and subtracts from the
/// target's health, scaled by the target's `DamageTakenModifier` (frostbite +10%, etc.; 1.0 when
/// absent). Also records the dealer in the target's `LastHitBy` (if it tracks one) for
/// kill-credit. Death is handled separately (per-entity death systems read `Health`).
pub fn apply_damage(
    mut events: EventReader<DamageEvent>,
    mut targets: Query<(&mut Health, Option<&mut LastHitBy>, Option<&DamageTakenModifier>)>,
) {
    for event in events.read() {
        if let Ok((mut health, last_hit_by, taken_mod)) = targets.get_mut(event.target) {
            let mult = taken_mod.map(|m| m.0).unwrap_or(1.0);
            health.current -= event.amount * mult;
            if let Some(mut last) = last_hit_by {
                last.0 = event.source;
            }
        }
    }
}
