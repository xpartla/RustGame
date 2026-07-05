// RunPlugin (Phase 7) — the encounter lifecycle, run-state resources, and act transitions.
//
// Joins `GameLogicPlugin`. Registers the ThroneRoom curse loader + the RoomModifiers resource, the
// EncounterCompleteEvent, and the lifecycle systems. RunState / CurrentEncounter are NOT inserted at
// build — the run-start flow (start_run: the windowed auto-start or Sim::start_run) inserts them.
// Every encounter system is gated on a live run (`resource_exists::<CurrentEncounter>` / `RunState`),
// so a runless world (the golden campaign) never touches any of it.

use bevy::prelude::*;

use crate::core::def_library::DefLibraryAppExt;
use crate::core::sets::CombatSet;
use crate::enemy::systems::death::enemy_death;
use crate::game::state::GameState;
use crate::run::state::{CurrentEncounter, RoomModifiers, RunState};
use crate::run::systems::select::handle_map_select;
use crate::run::systems::transitions::{
    check_objective, handle_encounter_complete, load_encounter, EncounterCompleteEvent,
};
use crate::world::graph::RoomModifierDef;

pub struct RunPlugin;

impl Plugin for RunPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoomModifiers>()
            // ThroneRoom curse defs (`.roommod.ron`) — loaded but inert until a ThroneRoom is entered.
            .register_def_library::<RoomModifierDef>()
            .add_event::<EncounterCompleteEvent>();

        // Encounter lifecycle — in the CombatSet::Death region (after enemy_death, so a killed boss
        // is despawned before the objective is counted). Gated on a live run so the golden campaign,
        // which never inserts CurrentEncounter, leaves every system inert (neutral by construction).
        app.add_systems(
            Update,
            (load_encounter, check_objective, handle_encounter_complete)
                .chain()
                .in_set(CombatSet::Death)
                .after(enemy_death)
                .run_if(in_state(GameState::InRun))
                .run_if(resource_exists::<CurrentEncounter>),
        );

        // Branch picker — runs only in the MapSelect overlay (the InRun world is frozen behind it).
        app.add_systems(
            Update,
            handle_map_select
                .run_if(in_state(GameState::MapSelect))
                .run_if(resource_exists::<RunState>),
        );
    }
}
