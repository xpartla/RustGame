// Local file persistence for MetaState and RunState.
//
// Current backend: RON serialization to a platform-appropriate app data directory.
// Future backend: cloud save / WASM local storage — swap the read/write functions here
// without touching MetaState or RunState structs (they're serde-compatible).
//
// File layout:
//   <app_data>/meta.ron       — MetaState (hero unlocks, scoreboard)
//   (RunState is stored inline in MetaState.in_progress_run as serialized bytes)
//
// Called from:
//   MetaPlugin::build: schedules load_meta in Startup.
//   run/systems/transitions.rs: calls save_meta after each node transition and on run end.
//
// TODO(Phase 8): implement. For now, meta is in-memory only (resets on app exit).

use bevy::prelude::*;
use crate::meta::state::MetaState;

/// Loads MetaState from disk into the Resource. Called once at startup.
/// If no save file exists, inserts a default MetaState (first run).
pub fn load_meta(mut commands: Commands) {
    // TODO(Phase 8): read from file; fall back to MetaState::default() on missing/corrupt file.
    commands.insert_resource(MetaState::default());
}

/// Saves the current MetaState to disk. Call after any mutation (run end, hero unlock).
/// Non-blocking in the future (async write); for now can be synchronous.
pub fn save_meta(_meta: Res<MetaState>) {
    todo!("Phase 8")
}

/// File path for meta.ron. Platform-aware.
/// TODO(Phase 8): use bevy's app data path API once available, or std::env::var("APPDATA").
pub fn meta_save_path() -> std::path::PathBuf {
    todo!("Phase 8")
}
