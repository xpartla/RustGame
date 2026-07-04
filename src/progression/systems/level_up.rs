// Consumes LevelUpEvent and drives the two-phase progression flow.
//
// On each LevelUpEvent received:
//   If phase == AbilityUnlock:
//     - Pop the next AbilityId from the appropriate band pool.
//     - Emit UnlockAbilityEvent (picked up by ability plugin to spawn the AbilityInstance).
//     - If both pools are now empty, transition phase to TalentChoices.
//   If phase == TalentChoices:
//     - Call generate_offer() with the current talent pool and RunRng.
//     - Store the result in LevelUpFlowState.pending_offer.
//     - Push GameState::TalentPicker.
//
// Level-banding: L2/L3 draw from band_2_3_remaining; L4+ draw from band_4_6_remaining.
// Determining which pool to draw from: check remaining lengths (2/3 pool first, then 4/6).
//
// Runs after CombatSet::Death (XP is awarded in Death, LevelUp fires after).

use bevy::prelude::*;
use crate::core::events::LevelUpEvent;
use crate::ability::components::UnlockAbilityEvent;
use crate::progression::state::{LevelUpFlowState, LevelUpPhase};
use crate::run::rng::RunRng;

/// TODO(Phase 2): implement.
/// Query: player entity with Experience + LevelUpFlowState; RunRng; TalentDef assets.
pub fn handle_level_up(
    mut _level_events: EventReader<LevelUpEvent>,
    mut _flow: ResMut<LevelUpFlowState>,
    mut _unlock_events: EventWriter<UnlockAbilityEvent>,
    mut _rng: ResMut<RunRng>,
    // + talent_defs, hero_def lookup, next_state writer
) {
    todo!("Phase 2")
}
