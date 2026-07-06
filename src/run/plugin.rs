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
use crate::run::systems::menu::{
    handle_character_select_input, handle_login_input, handle_main_menu_input,
    handle_scoreboard_input,
};
use crate::run::systems::persistence::{apply_resume_request, tick_run_timer, ResumeRunRequest};
use crate::run::systems::reset::{
    apply_start_run_request, handle_game_over_input, StartRunRequest,
};
use crate::run::systems::select::handle_map_select;
use crate::run::systems::transitions::{
    check_objective, enter_merchant, handle_encounter_complete, load_encounter, EncounterCompleteEvent,
};
use crate::world::graph::RoomModifierDef;

pub struct RunPlugin;

impl Plugin for RunPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoomModifiers>()
            // ThroneRoom curse defs (`.roommod.ron`) — loaded but inert until a ThroneRoom is entered.
            .register_def_library::<RoomModifierDef>()
            .add_event::<EncounterCompleteEvent>()
            .add_event::<StartRunRequest>()
            .add_event::<ResumeRunRequest>();

        // Deterministic run clock (Phase 8, D2) — feeds the scoreboard's speed bonus. Gated on a
        // live run so the golden campaign (which never starts one) never ticks it.
        app.add_systems(
            Update,
            tick_run_timer
                .run_if(in_state(GameState::InRun))
                .run_if(resource_exists::<RunState>),
        );

        // Encounter lifecycle — in the CombatSet::Death region (after enemy_death, so a killed boss
        // is despawned before the objective is counted). Gated on a live run so the golden campaign,
        // which never inserts CurrentEncounter, leaves every system inert (neutral by construction).
        app.add_systems(
            Update,
            (load_encounter, check_objective, handle_encounter_complete, enter_merchant)
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

        // Run reset / restart (Phase 7.5B). `handle_game_over_input` reads R/M on the death screen;
        // `apply_start_run_request` performs the exclusive-world reset only on a frame carrying a
        // request (so it is inert — and neutral — in the runless campaign).
        app.add_systems(
            Update,
            handle_game_over_input.run_if(in_state(GameState::GameOver)),
        );
        app.add_systems(
            Update,
            apply_start_run_request.run_if(on_event::<StartRunRequest>),
        );

        // Resume Run (Phase 8, §3.2) — the mirror of apply_start_run_request. Only runs on a frame
        // carrying a ResumeRunRequest (the main-menu Resume input, gated there on a save existing).
        app.add_systems(
            Update,
            apply_resume_request.run_if(on_event::<ResumeRunRequest>),
        );

        // Login + main menu + character select (Phase 7.5C; Login/Resume/Scoreboard Phase 8). Each
        // is gated on its own state; the campaign never enters any of them ⇒ all inert there.
        // Selecting a hero emits a StartRunRequest, handled by the shared reset path above.
        app.add_systems(Update, handle_login_input.run_if(in_state(GameState::Login)));
        app.add_systems(
            Update,
            handle_main_menu_input.run_if(in_state(GameState::Menu)),
        );
        app.add_systems(
            Update,
            handle_character_select_input.run_if(in_state(GameState::CharacterSelect)),
        );
        app.add_systems(
            Update,
            handle_scoreboard_input.run_if(in_state(GameState::Scoreboard)),
        );
    }
}
