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

pub mod plugin;
pub mod rng;
pub mod state;
pub mod systems;
