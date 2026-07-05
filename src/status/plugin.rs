// StatusPlugin — wires the status effect system into the app (Phase 3).
//
// Responsibilities:
//   - Registers StatusEffectDef as a Bevy asset + its `.status.ron` loader, and StatusLibrary.
//   - Registers ApplyStatusEvent / RemoveStatusEvent.
//   - Loads the six status RON files at startup.
//   - StatusSet::Tick:         despawn_orphaned_status → apply_status_effects → tick_status_effects
//   - StatusSet::CrossInteract: apply_cross_interactions → remove_status_effects
//   - resolve_actor_status (CC + stat modifiers) is added in Phase 3C.
//
// The set chain (Damage → Apply → Tick → CrossInteract → Death) is configured in CorePlugin.
// All systems run in InState(GameState::InRun): status ticks freeze with the world behind overlays.

use bevy::prelude::*;
use crate::core::def_library::DefLibraryAppExt;
use crate::core::events::AddGameplayEventExt;
use crate::core::sets::StatusSet;
use crate::game::state::GameState;
use crate::status::assets::StatusEffectDef;
use crate::status::components::{ApplyStatusEvent, RemoveStatusEvent};
use crate::status::systems::apply::apply_status_effects;
use crate::status::systems::cross_interact::apply_cross_interactions;
use crate::status::systems::remove::{despawn_orphaned_status, remove_status_effects};
use crate::status::systems::resolve::resolve_actor_status;
use crate::status::systems::tick::tick_status_effects;

pub struct StatusPlugin;

impl Plugin for StatusPlugin {
    fn build(&self, app: &mut App) {
        // StatusEffectDef asset + `.status.ron` loader + StatusLibrary + Startup populate.
        app.register_def_library::<StatusEffectDef>()
            // Combat-resolution events: preserved across overlay states (see AddGameplayEventExt).
            .add_gameplay_event::<ApplyStatusEvent>()
            .add_gameplay_event::<RemoveStatusEvent>();

        app.add_systems(
            Update,
            (despawn_orphaned_status, apply_status_effects, tick_status_effects)
                .chain()
                .in_set(StatusSet::Tick)
                .run_if(in_state(GameState::InRun)),
        );
        app.add_systems(
            Update,
            (apply_cross_interactions, remove_status_effects, resolve_actor_status)
                .chain()
                .in_set(StatusSet::CrossInteract)
                .run_if(in_state(GameState::InRun)),
        );
    }
}
