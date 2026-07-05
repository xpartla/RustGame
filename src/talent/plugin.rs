// TalentPlugin — wires the talent system into the app (Phase 2).
//
// Responsibilities:
//   - Registers TalentDef as a Bevy asset + its RON loader (*.talent.ron).
//   - Registers TalentLibrary (id → handle) and loads the talent RON files at startup.
//   - Attaches AcquiredTalents / ActiveHooks to the player on spawn.
//   - Registers TalentAcquiredEvent / TalentRemovedEvent and the install/uninstall systems.
//
// The install/uninstall systems run ungated by GameState: TalentAcquiredEvent is emitted from
// the TalentPicker state, so its reader must not be frozen with the InRun world.

use bevy::prelude::*;
use crate::core::def_library::DefLibraryAppExt;
use crate::talent::assets::TalentDef;
use crate::talent::systems::apply::{
    attach_talent_components, install_acquired_talent, uninstall_removed_talent,
    TalentAcquiredEvent, TalentRemovedEvent,
};

pub struct TalentPlugin;

impl Plugin for TalentPlugin {
    fn build(&self, app: &mut App) {
        // TalentDef asset + RON loader + TalentLibrary + Startup populate, in one call.
        app.register_def_library::<TalentDef>()
            .add_event::<TalentAcquiredEvent>()
            .add_event::<TalentRemovedEvent>();

        // attach runs in Update (not Startup) so `Added<Player>` reliably fires after the
        // Startup `spawn_player` — Startup system ordering relative to it is otherwise undefined.
        app.add_systems(
            Update,
            (attach_talent_components, install_acquired_talent, uninstall_removed_talent),
        );
    }
}
