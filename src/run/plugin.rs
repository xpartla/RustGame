// TODO(Phase 7): Wire into GamePlugin. RunRng alone can be introduced in Phase 0.
//
// Responsibilities:
//   - Registers EncounterCompleteEvent
//   - Adds handle_encounter_complete system
//   - RunState and RunRng are inserted/removed dynamically (not in plugin build):
//     inserted by the "start run" or "resume run" flow in meta/
//     removed (or replaced) on game-over or return to menu

use bevy::prelude::*;

pub struct RunPlugin;

impl Plugin for RunPlugin {
    fn build(&self, _app: &mut App) {
        todo!("Phase 7 (RunRng registration alone can happen in Phase 0)")
    }
}
