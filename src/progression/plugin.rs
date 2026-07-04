// ProgressionPlugin — the leveling & talent-offer flow (Phase 2).
//
// Responsibilities:
//   - Inserts LevelUpFlowState at startup (band pools shuffled with RunRng).
//   - Registers ThroneRoomRewardEvent (consumer lands in Phase 7).
//   - handle_level_up: consumes LevelUpEvent after gain_experience, drives phase transitions,
//     emits UnlockAbilityEvent, and enters the TalentPicker overlay when a choice is owed.
//   - refill_offer + handle_talent_choice: drain the owed-choice backlog through the overlay.
//   - debug_force_level_up (dev builds): `L` to fast-forward a level.

use bevy::prelude::*;
use crate::game::state::GameState;
use crate::player::systems::experience::gain_experience;
use crate::progression::systems::level_up::{handle_level_up, init_level_flow};
use crate::progression::systems::offer::{handle_talent_choice, refill_offer, ThroneRoomRewardEvent};

pub struct ProgressionPlugin;

impl Plugin for ProgressionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ThroneRoomRewardEvent>();

        app.add_systems(Startup, init_level_flow);

        app.add_systems(
            Update,
            handle_level_up
                .after(gain_experience)
                .run_if(in_state(GameState::InRun)),
        );

        app.add_systems(
            Update,
            (refill_offer, handle_talent_choice)
                .chain()
                .run_if(in_state(GameState::TalentPicker)),
        );

        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            crate::progression::systems::level_up::debug_force_level_up
                .run_if(in_state(GameState::InRun)),
        );
    }
}
