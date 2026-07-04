// Talent offer presentation and player choice handling.
//
// This system runs in GameState::TalentPicker. It reads LevelUpFlowState.pending_offer
// and sends offer data to the UI. On player choice, it emits TalentAcquiredEvent.
//
// The UI system (ui/screens/talent_offer.rs) reads from the same pending_offer and
// renders the 3-option picker. This system only handles the event bridge.
//
// Player choice:
//   - Pick option N (1/2/3): emit TalentAcquiredEvent for options[N].
//   - Decline (press Escape or "Skip"): emit no event, just clear pending_offer.
//   In both cases: clear pending_offer, pop GameState back to InRun.
//
// ThroneRoom flow:
//   The encounter system emits a ThroneRoomRewardEvent. This system picks it up,
//   generates an offer with OfferContext::ThroneRoom, stores in pending_offer,
//   and pushes GameState::TalentPicker. The UI is identical to the normal offer screen.

use bevy::prelude::*;
use crate::talent::systems::apply::TalentAcquiredEvent;
use crate::progression::state::LevelUpFlowState;

#[derive(Event, Debug)]
pub struct ThroneRoomRewardEvent {
    pub owner: Entity,
}

/// TODO(Phase 2): implement — handles player choice from the TalentPicker screen.
pub fn handle_talent_choice(
    // keyboard input, LevelUpFlowState, TalentAcquiredEvent writer, NextState
    mut _flow: ResMut<LevelUpFlowState>,
    mut _acquired: EventWriter<TalentAcquiredEvent>,
) {
    todo!("Phase 2")
}

/// TODO(Phase 7): implement — handles ThroneRoom reward trigger.
pub fn handle_throne_room_reward(
    mut _events: EventReader<ThroneRoomRewardEvent>,
    mut _flow: ResMut<LevelUpFlowState>,
    // + RunRng, talent pool sources, NextState::TalentPicker
) {
    todo!("Phase 7")
}
