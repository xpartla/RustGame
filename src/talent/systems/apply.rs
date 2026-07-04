// Installs and removes ActiveHook components when Behavior talents are gained or removed.
//
// Listen for:
//   TalentAcquiredEvent  — adds talent to AcquiredTalents; if effect == Behavior(id),
//                          pushes id into ActiveHooks
//   TalentRemovedEvent   — removes one copy from AcquiredTalents; if effect == Behavior(id)
//                          and count drops to 0, pops id from ActiveHooks
//
// These events are emitted by:
//   - progression/systems/offer.rs on player choice (TalentAcquiredEvent)
//   - talent/systems/merchant.rs on remove-talent and trade-up (TalentRemovedEvent)
//
// Modifier talents (TalentEffect::Modifier) do NOT need component installation — they
// are evaluated on-the-fly by resolve_params() at ability fire time.

use bevy::prelude::*;
use crate::talent::assets::{TalentDef, TalentEffect, TalentId};
use crate::talent::components::{AcquiredTalents, ActiveHooks};

#[derive(Event, Debug)]
pub struct TalentAcquiredEvent {
    pub owner: Entity,
    pub talent_id: TalentId,
}

#[derive(Event, Debug)]
pub struct TalentRemovedEvent {
    pub owner: Entity,
    pub talent_id: TalentId,
}

/// TODO(Phase 2): implement.
/// Query: player with (AcquiredTalents, ActiveHooks); reads TalentDef assets.
pub fn install_acquired_talent(
    mut _events: EventReader<TalentAcquiredEvent>,
    mut _players: Query<(&mut AcquiredTalents, &mut ActiveHooks)>,
    _talent_defs: Res<Assets<TalentDef>>,
) {
    todo!("Phase 2")
}

/// TODO(Phase 2): implement.
pub fn uninstall_removed_talent(
    mut _events: EventReader<TalentRemovedEvent>,
    mut _players: Query<(&mut AcquiredTalents, &mut ActiveHooks)>,
    _talent_defs: Res<Assets<TalentDef>>,
) {
    todo!("Phase 2")
}
