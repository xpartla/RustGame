use bevy::prelude::{EventReader, Query};
use crate::core::components::Health;
use crate::core::events::DamageEvent;

/// The single point that mutates `Health`. Drains `DamageEvent`s and subtracts from the
/// target's health. Death is handled separately (per-entity death systems read `Health`).
pub fn apply_damage(
    mut events: EventReader<DamageEvent>,
    mut healths: Query<&mut Health>,
) {
    for event in events.read() {
        if let Ok(mut health) = healths.get_mut(event.target) {
            health.current -= event.amount;
        }
    }
}
