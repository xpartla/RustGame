use bevy::prelude::{EventReader, Query};
use crate::core::components::{Health, HealingTakenModifier};
use crate::core::events::HealEvent;

/// The single point that *adds* to `Health` (the healing counterpart to `apply_damage`).
/// Drains `HealEvent`s and restores the target's health, scaled by the target's
/// `HealingTakenModifier` (Phase 9.2 — `bdk_passive_health_and_healing`; 1.0 when absent), clamping
/// to `Health.max` so a pickup can never overheal. The separate 35% heal CAP
/// (`bdk_passive_no_heal_cap`) is enforced afterward by
/// `talent::systems::passives::enforce_heal_cap`, not here.
pub fn apply_heal(
    mut events: EventReader<HealEvent>,
    mut healths: Query<(&mut Health, Option<&HealingTakenModifier>)>,
) {
    for event in events.read() {
        if let Ok((mut health, taken_mod)) = healths.get_mut(event.target) {
            let taken_mult = taken_mod.map(|m| m.0).unwrap_or(1.0);
            health.current = (health.current + event.amount * taken_mult).min(health.max);
        }
    }
}
