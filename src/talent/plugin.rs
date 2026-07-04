// TODO(Phase 2): Wire into GamePlugin.
//
// Responsibilities:
//   - Registers TalentDef as a Bevy asset + loader
//   - Registers TalentAcquiredEvent, TalentRemovedEvent
//   - Registers MerchantRemoveRequest, MerchantTradeRequest
//   - Adds install_acquired_talent, uninstall_removed_talent systems

use bevy::prelude::*;

pub struct TalentPlugin;

impl Plugin for TalentPlugin {
    fn build(&self, _app: &mut App) {
        todo!("Phase 2")
    }
}
