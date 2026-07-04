// Merchant operations: talent removal and 3-for-1 trade-up.
//
// Remove talent:
//   - Emits TalentRemovedEvent for the selected talent.
//   - talent/systems/apply.rs picks up the event and updates AcquiredTalents / ActiveHooks.
//   - UI confirms, then merchant screen returns to map graph.
//
// 3-for-1 trade-up:
//   - Player selects 3 talents to sacrifice.
//   - Emits TalentRemovedEvent x3.
//   - Calls progression::offer::generate_offer with OfferContext::MerchantTradeUp
//     using min_rarity = one step above the highest rarity sacrificed.
//   - The resulting offer is presented as a 1-of-N pick (same TalentOffer flow).
//
// Merchant interaction is triggered by GameState::Merchant; this system runs only in that state.

use bevy::prelude::*;
use crate::talent::assets::TalentId;
use crate::talent::systems::apply::TalentRemovedEvent;

/// Events from the merchant UI screen.
#[derive(Event, Debug)]
pub struct MerchantRemoveRequest {
    pub owner: Entity,
    pub talent_id: TalentId,
}

#[derive(Event, Debug)]
pub struct MerchantTradeRequest {
    pub owner: Entity,
    pub sacrifice: [TalentId; 3],
}

/// TODO(Phase 8): implement both operations.
pub fn handle_merchant_remove(
    mut _requests: EventReader<MerchantRemoveRequest>,
    mut _removed_events: EventWriter<TalentRemovedEvent>,
) {
    todo!("Phase 8")
}

pub fn handle_merchant_trade(
    mut _requests: EventReader<MerchantTradeRequest>,
    mut _removed_events: EventWriter<TalentRemovedEvent>,
    // + offer generator call + push GameState::TalentPicker
) {
    todo!("Phase 8")
}
