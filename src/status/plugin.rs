// TODO(Phase 3): Wire into GamePlugin.
//
// Responsibilities:
//   - Registers StatusEffectDef as a Bevy asset + loader
//   - Registers ApplyStatusEvent, RemoveStatusEvent
//   - Adds apply_status_effects in StatusSet::Apply
//   - Adds tick_status_effects in StatusSet::Tick
//   - Adds apply_cross_interactions in StatusSet::CrossInteract
//   - All systems run in InState(GameState::InRun)

use bevy::prelude::*;

pub struct StatusPlugin;

impl Plugin for StatusPlugin {
    fn build(&self, _app: &mut App) {
        todo!("Phase 3")
    }
}
