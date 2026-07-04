// Encounter lifecycle and act transitions.
//
// Encounter complete → next node:
//   1. Objective fulfilled (kill all / survive timer / boss dead) → emit EncounterCompleteEvent.
//   2. Save RunState to MetaState.in_progress_run via meta/persistence.rs.
//   3. Present the player with reachable next nodes (the map graph branches).
//   4. Player selects a node → update RunState.current_node → load next encounter.
//
// Act transitions:
//   On reaching the act boss and defeating it, increment RunState.current_act and generate
//   the next act's graph from RunRng.
//
// Run end:
//   On act 3 boss defeat: run complete → write RunRecord to MetaState → GameState::GameOver.
//   On player death:      run failed  → write RunRecord → GameState::GameOver.
//
// Interactions:
//   - world/systems: emits EncounterCompleteEvent when the objective is met.
//   - meta/persistence.rs: called here to serialize RunState.
//   - GameState transitions driven by this system.

use bevy::prelude::*;

#[derive(Event, Debug)]
pub struct EncounterCompleteEvent;

/// TODO(Phase 7): implement.
pub fn handle_encounter_complete(
    mut _events: EventReader<EncounterCompleteEvent>,
    // + RunState, RunRng, MetaState, GameState next state
) {
    todo!("Phase 7")
}
