// Element cancellation: fire removes frostbite, frost removes blaze (and vice versa).
//
// This system listens to DamageEvent (specifically the `tags` field added in Phase 0) and
// checks each active StatusEffectInstance on the damaged target. If the instance's
// StatusEffectDef.removed_by_tags contains a tag from the event, it emits RemoveStatusEvent.
//
// Because the cancellation rules are encoded in the RON files (not in this code), adding
// a new element that cancels an existing effect only requires updating the RON file.
// No code change here.
//
// Runs in StatusSet::CrossInteract (after StatusSet::Tick, before CombatSet::Death).

use bevy::prelude::*;
use crate::core::events::DamageEvent;
use crate::status::assets::StatusEffectDef;
use crate::status::components::{RemoveStatusEvent, StatusEffectInstance};

/// TODO(Phase 3): implement.
/// For each DamageEvent, find all StatusEffectInstances on the target whose
/// removed_by_tags intersects with the event's tags, emit RemoveStatusEvent for each.
pub fn apply_cross_interactions(
    mut _damage_events: EventReader<DamageEvent>,
    _instances: Query<(&StatusEffectInstance, &Parent)>,
    _effect_defs: Res<Assets<StatusEffectDef>>,
    mut _remove_events: EventWriter<RemoveStatusEvent>,
) {
    todo!("Phase 3")
}
