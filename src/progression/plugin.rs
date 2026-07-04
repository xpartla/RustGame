// TODO(Phase 2): Wire into GamePlugin.
//
// Responsibilities:
//   - Inserts LevelUpFlowState resource (initially empty; populated when a run starts)
//   - Registers ThroneRoomRewardEvent
//   - Adds handle_level_up after CombatSet::Death
//   - Adds handle_talent_choice in InState(GameState::TalentPicker)
//   - Adds handle_throne_room_reward in InState(GameState::InRun)

use bevy::prelude::*;

pub struct ProgressionPlugin;

impl Plugin for ProgressionPlugin {
    fn build(&self, _app: &mut App) {
        todo!("Phase 2")
    }
}
