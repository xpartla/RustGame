use bevy::prelude::{EventReader, Query};
use crate::core::components::Health;
use crate::core::events::HealEvent;

/// The single point that *adds* to `Health` (the healing counterpart to `apply_damage`).
/// Drains `HealEvent`s and restores the target's health, clamping to `Health.max` so a pickup
/// can never overheal.
pub fn apply_heal(
    mut events: EventReader<HealEvent>,
    mut healths: Query<&mut Health>,
) {
    for event in events.read() {
        if let Ok(mut health) = healths.get_mut(event.target) {
            health.current = (health.current + event.amount).min(health.max);
        }
    }
}
