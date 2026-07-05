// Removing status effects.
//
//   remove_status_effects   — consumes RemoveStatusEvent (element cancellation, talent
//                             consumption), despawns every matching instance. Deduped so two
//                             cancellation events for the same effect never double-despawn.
//   despawn_orphaned_status — reaps instances whose target has despawned. Instances are
//                             top-level entities (not children), so enemy_death's non-recursive
//                             despawn leaves them behind; this sweep cleans up. A target always
//                             has Health, so a missing Health means the target is gone.

use bevy::prelude::*;
use std::collections::HashSet;
use crate::core::components::Health;
use crate::status::components::{RemoveStatusEvent, StatusEffectInstance};

pub fn remove_status_effects(
    mut events: EventReader<RemoveStatusEvent>,
    mut commands: Commands,
    instances: Query<(Entity, &StatusEffectInstance)>,
) {
    let mut to_despawn: HashSet<Entity> = HashSet::new();
    for ev in events.read() {
        for (e, inst) in &instances {
            if inst.target == ev.target && inst.def_id == ev.effect_id {
                to_despawn.insert(e);
            }
        }
    }
    for e in to_despawn {
        commands.entity(e).try_despawn();
    }
}

pub fn despawn_orphaned_status(
    mut commands: Commands,
    instances: Query<(Entity, &StatusEffectInstance)>,
    alive: Query<(), With<Health>>,
) {
    for (e, inst) in &instances {
        if alive.get(inst.target).is_err() {
            commands.entity(e).try_despawn();
        }
    }
}
