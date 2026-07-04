// TODO(Phase 8): Wire into GamePlugin. load_meta alone can be added in Phase 0.
//
// Responsibilities:
//   - Schedules load_meta in Startup (inserts MetaState Resource)
//   - Registers save_meta as a one-shot system callable by run/systems/transitions.rs

use bevy::prelude::*;
use crate::meta::persistence::load_meta;

pub struct MetaPlugin;

impl Plugin for MetaPlugin {
    fn build(&self, app: &mut App) {
        // load_meta is safe to add in Phase 0 — it just inserts MetaState::default().
        // Replace with the real file-loading version in Phase 8.
        app.add_systems(Startup, load_meta);
    }
}
