use bevy::prelude::ResMut;
use crate::run::rng::RunRng;
use crate::world::components::TileMap;
use crate::world::generator::procedural_room_layout;

/// Procedurally fills the `TileMap` at Startup by delegating to `procedural_room_layout`
/// (world/generator.rs) — the same border-ring + random-walk-blob blob, just repositioned as the
/// per-room generator's `Map` layout. Kept as a thin Startup caller so the windowed game has a map
/// before a run starts (Phase 7's `load_encounter` regenerates it per encounter). The RunRng draw
/// order is unchanged from the old inline blob, so the golden master is byte-identical.
///
/// Draws from `RunRng` (the seeded run stream) rather than `thread_rng`, so the layout is
/// reproducible from the run seed.
pub fn generate_map(mut map: ResMut<TileMap>, mut run_rng: ResMut<RunRng>) {
    procedural_room_layout(&mut map, &mut run_rng);
}
