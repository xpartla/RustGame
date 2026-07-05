// Phase 7: Run state — authoritative per-run data and seeded RNG.
//
// NOT yet wired into main.rs or GamePlugin. Add in Phase 7 (graph/room work) although
// RunRng should be introduced in Phase 0 as a prerequisite for seeded map generation.
//
// Two resources live here:
//   RunState — everything needed to resume a run: seed, graph position, hero, levels, talents.
//   RunRng   — the seeded SmallRng. ONLY run-deterministic systems consume from this.
//              Non-deterministic systems (VFX, audio variation) use rand::thread_rng().
//
// Invariant: RunState is serialized on every node transition. Deserialized on "Resume Run".
// RunRng state is included in the serialized blob so a resumed run picks up mid-stream.
//
// Module map:
//   state.rs  — RunState resource
//   rng.rs    — RunRng resource
//   systems/
//     transitions.rs — encounter-complete → next-node selection, act transitions, game-over

// Phase 7 status: the whole run module is live — `RunState` + `CurrentEncounter` (in-memory
// resources; serde is Phase 8), `RunPlugin` (joins `GameLogicPlugin`), and the encounter-lifecycle
// systems (start/load/objective/advance/select). RunState/CurrentEncounter are inserted only by the
// run-start flow, so a runless world (the golden campaign) never touches any of it.
pub mod rng;
pub mod state;
pub mod plugin;
pub mod systems;

pub use plugin::RunPlugin;
