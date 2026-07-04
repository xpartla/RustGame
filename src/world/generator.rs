// Room layout generators — produce TileMap content for a given encounter node.
//
// The primary switch is on EncounterType:
//   Map       → procedural_room_layout (blob scatter, may reuse generate_map.rs logic)
//   BossRoom  → boss_room_layout (more open floor, defined arena)
//   ThroneRoom → throne_room_layout (distinct geometry: hall + raised platform)
//   ActBoss   → act_boss_layout (large open area)
//   Merchant  → merchant_layout (small safe room; no enemies)
//
// All generators take &mut RunRng to stay seed-deterministic.
// The existing generate_map.rs blob algorithm can serve as the interior scatter step
// within procedural_room_layout; it is not discarded but repositioned as a sub-call.
//
// Interactions:
//   - world/systems: calls the appropriate generator when a new encounter is loaded.
//   - world/components.rs TileMap: cleared and repopulated by each generator.
//   - run/rng.rs RunRng: consumed by all generators; no thread_rng() calls here.

use crate::run::rng::RunRng;
use crate::world::components::TileMap;
use crate::world::graph::EncounterType;

/// Entry point: generates the TileMap for the given encounter type.
/// TODO(Phase 7): implement. Dispatches to the appropriate sub-generator.
pub fn generate_room(encounter: &EncounterType, map: &mut TileMap, rng: &mut RunRng) {
    match encounter {
        EncounterType::Map { .. } => procedural_room_layout(map, rng),
        EncounterType::BossRoom => boss_room_layout(map, rng),
        EncounterType::ThroneRoom => throne_room_layout(map, rng),
        EncounterType::ActBoss => act_boss_layout(map, rng),
        EncounterType::Merchant => merchant_layout(map),
    }
}

/// Standard procedural room. Interior obstacle scatter (reuses blob logic from generate_map.rs).
fn procedural_room_layout(map: &mut TileMap, rng: &mut RunRng) {
    todo!("Phase 7: border ring + random interior obstacles from RunRng (port generate_map.rs blob logic)")
}

/// More open arena for boss fights. Fewer obstacles, clear sightlines.
fn boss_room_layout(map: &mut TileMap, rng: &mut RunRng) {
    todo!("Phase 7")
}

/// Throne room: hall with raised dais geometry. Distinct from normal rooms.
/// The distinct geometry is part of the kiss/curse fantasy — player can see the threat.
fn throne_room_layout(map: &mut TileMap, rng: &mut RunRng) {
    todo!("Phase 7")
}

/// Large open area for act boss fights.
fn act_boss_layout(map: &mut TileMap, _rng: &mut RunRng) {
    todo!("Phase 7")
}

/// Small safe room with no obstacles and no enemies.
fn merchant_layout(map: &mut TileMap) {
    todo!("Phase 7")
}
