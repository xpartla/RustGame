use bevy::prelude::{Commands, EventReader, Query};
use crate::core::components::Absorb;
use crate::core::events::GainShieldEvent;

/// The single point that grants `Absorb` shields (Phase 9.1, §8.1(5)). Additive: a target that
/// already carries an `Absorb` gets the amount added to its pool; otherwise the component is
/// inserted fresh. Mirrors `apply_heal`'s single-consumer shape.
pub fn apply_shield_gain(
    mut commands: Commands,
    mut events: EventReader<GainShieldEvent>,
    mut targets: Query<&mut Absorb>,
) {
    for event in events.read() {
        if let Ok(mut absorb) = targets.get_mut(event.target) {
            absorb.amount += event.amount;
        } else {
            commands.entity(event.target).insert(Absorb { amount: event.amount });
        }
    }
}
