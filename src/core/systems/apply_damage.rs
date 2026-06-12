use bevy::prelude::{EventReader, Query};
use crate::core::components::{Health, LastHitBy};
use crate::core::events::DamageEvent;

/// The single point that mutates `Health`. Drains `DamageEvent`s and subtracts from the
/// target's health. Also records the dealer in the target's `LastHitBy` (if it tracks one) for
/// kill-credit. Death is handled separately (per-entity death systems read `Health`).
pub fn apply_damage(
    mut events: EventReader<DamageEvent>,
    mut targets: Query<(&mut Health, Option<&mut LastHitBy>)>,
) {
    for event in events.read() {
        if let Ok((mut health, last_hit_by)) = targets.get_mut(event.target) {
            health.current -= event.amount;
            if let Some(mut last) = last_hit_by {
                last.0 = event.source;
            }
        }
    }
}
