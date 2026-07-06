// Account-level meta state — hero unlocks and scoreboard (Phase 8).
//
// Joins the crate (lib.rs) and GameLogicPlugin (via MetaPlugin) in Phase 8.
//
// Deliberately decoupled from `run`. MetaState is inserted unconditionally at app startup
// and persists across all GameState transitions (including game-over and return to menu).
// RunState is inserted only when a run begins and removed on game-over/victory.
//
// Persistence: serialized to a local file via serde (RON). The format is serde-compatible so
// the backend can be replaced (cloud, WASM) without touching the data structures.
//
// Module map:
//   state.rs       — MetaState resource, RunRecord, SavedRun, hero_is_unlocked
//   persistence.rs — serialize_meta/deserialize_meta (pure) + save_meta_to_disk/
//                    load_meta_from_disk (thin disk wrappers), path resolution
//   plugin.rs      — MetaPlugin (inserts the resource; sim-able, no disk)
//   score.rs       — the scoreboard's pure score formula

pub mod persistence;
pub mod plugin;
pub mod score;
pub mod state;
