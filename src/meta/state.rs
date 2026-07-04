// MetaState — account-level state that outlives any single run.
//
// Lives as a Resource inserted unconditionally at app startup (MetaPlugin::build).
// Persists across GameState transitions, including game-over and return to menu.
// Contains NO run-specific data — that lives in RunState.
//
// Persistence: serialized to a local file by meta/persistence.rs.
// Format: serde (currently via RON; backend-swappable for future WASM/cloud save).
//
// Power does NOT persist between runs: no currency, no permanent stat upgrades.
// Only hero unlocks and scoreboard entries carry over.
//
// Interactions:
//   - meta/persistence.rs: load_meta() at startup, save_meta() on run end.
//   - run/systems/transitions.rs: appends RunRecord on run end, clears in_progress_run.
//   - ui/screens/character_select.rs: reads unlocked_heroes to grey out locked heroes.
//   - ui/screens/scoreboard.rs: reads run_history.

use crate::hero::assets::HeroId;
use bevy::prelude::*;
use std::collections::HashSet;

/// Inserted at app startup; never removed. Serialized to disk.
#[derive(Resource, Debug, Clone, Default)]
pub struct MetaState {
    /// Heroes the player has unlocked. All heroes start locked; Blood Death Knight (or the
    /// first defined hero) is unlocked by default at first launch.
    pub unlocked_heroes: HashSet<HeroId>,
    /// Completed run records, newest first. Used for the scoreboard screen.
    pub run_history: Vec<RunRecord>,
    /// If Some, there is a run in progress. "Resume Run" deserializes this.
    /// Cleared on run completion or game-over.
    pub in_progress_run: Option<Vec<u8>>, // serialized RunState bytes
}

#[derive(Debug, Clone)]
pub struct RunRecord {
    pub hero_id: HeroId,
    /// Act reached before run ended (1–3; 3 = reached act 3 boss).
    pub act_reached: u8,
    pub score: u32,
    /// Unix timestamp (seconds since epoch) of run end.
    pub timestamp_unix: u64,
}
