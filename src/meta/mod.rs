// Phase 8: Account-level meta state — hero unlocks and scoreboard.
//
// NOT yet wired into main.rs or GamePlugin. Add in Phase 8.
//
// Deliberately decoupled from `run`. MetaState is inserted unconditionally at app startup
// and persists across all GameState transitions (including game-over and return to menu).
// RunState is inserted only when a run begins and removed on game-over.
//
// Persistence: serialized to a local file via serde. The format is serde-compatible so
// the backend can be replaced (cloud, WASM) without touching the data structures.
//
// Module map:
//   state.rs      — MetaState resource, RunRecord, SavedRunState
//   persistence.rs — load_meta() / save_meta() — file I/O, path resolution, versioning

pub mod persistence;
pub mod plugin;
pub mod state;
