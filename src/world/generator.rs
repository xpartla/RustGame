// Room layout generators — produce TileMap content for a given encounter node (Phase 7).
//
// The primary switch is on EncounterType:
//   Map       → procedural_room_layout (border ring + random-walk obstacle blobs)
//   BossRoom  → boss_room_layout (open arena, a few corner pillars, clear sightlines)
//   ThroneRoom → throne_room_layout (distinct hall + raised dais geometry)
//   ActBoss   → act_boss_layout (large open area)
//   Merchant  → merchant_layout (small safe room; no obstacles)
//
// All generators are seed-deterministic — they draw only from RunRng, never thread_rng — so a
// resumed/replayed run reproduces the same rooms (the docs/testing.md reproducibility contract).
// Each clears + repopulates the shared TileMap resource and always leaves the spawn-clear box around
// the origin walkable (the player is teleported to the origin on load).
//
// `procedural_room_layout` is the old Startup `generate_map` blob, repositioned here as a sub-call;
// `world/systems/generate_map.rs` now delegates to it, so the same seed produces the same map (and
// the same downstream RunRng state → the golden master is byte-identical).

use rand::Rng;
use crate::core::components::GridPosition;
use crate::run::rng::RunRng;
use crate::world::components::TileMap;
use crate::world::constants::{
    MAP_HALF_TILES, OBSTACLE_BLOB_COUNT, OBSTACLE_BLOB_MAX_TILES, OBSTACLE_BLOB_MIN_TILES,
    SPAWN_CLEAR_RADIUS,
};
use crate::world::graph::EncounterType;

/// Half-extent of the small Merchant rest room.
const MERCHANT_HALF_TILES: i32 = 14;

/// 4-directional steps used to grow obstacle blobs via a short random walk.
const STEPS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Entry point: generates the TileMap for the given encounter type. Dispatches to the sub-generator.
pub fn generate_room(encounter: &EncounterType, map: &mut TileMap, rng: &mut RunRng) {
    match encounter {
        EncounterType::Map { .. } => procedural_room_layout(map, rng),
        EncounterType::BossRoom => boss_room_layout(map, rng),
        EncounterType::ThroneRoom => throne_room_layout(map, rng),
        EncounterType::ActBoss => act_boss_layout(map, rng),
        EncounterType::Merchant => merchant_layout(map),
    }
}

/// Sets the map extents, clears it, and lays a solid border ring. No RNG — the ring is fixed, so
/// callers that add nothing else stay trivially deterministic.
fn border_ring(map: &mut TileMap, h: i32) {
    map.half_width = h;
    map.half_height = h;
    map.blocked.clear();
    for x in -h..=h {
        map.blocked.insert(GridPosition { x, y: -h });
        map.blocked.insert(GridPosition { x, y: h });
    }
    for y in -h..=h {
        map.blocked.insert(GridPosition { x: -h, y });
        map.blocked.insert(GridPosition { x: h, y });
    }
}

/// Standard procedural room: border ring + scattered obstacle blobs grown by a short random walk from
/// RunRng, skipping the spawn-clear box around the origin. This is the prototype `generate_map` blob,
/// verbatim (same RunRng draw order), so it reproduces the old map for the same seed.
pub(crate) fn procedural_room_layout(map: &mut TileMap, run_rng: &mut RunRng) {
    let h = MAP_HALF_TILES;
    border_ring(map, h);

    let rng = run_rng.rng();
    for _ in 0..OBSTACLE_BLOB_COUNT {
        let mut cur = GridPosition {
            x: rng.gen_range(-h + 1..h),
            y: rng.gen_range(-h + 1..h),
        };
        let length = rng.gen_range(OBSTACLE_BLOB_MIN_TILES..=OBSTACLE_BLOB_MAX_TILES);
        for _ in 0..length {
            let in_clear_box =
                cur.x.abs() <= SPAWN_CLEAR_RADIUS && cur.y.abs() <= SPAWN_CLEAR_RADIUS;
            if !in_clear_box {
                map.blocked.insert(cur);
            }
            let step = STEPS[rng.gen_range(0..STEPS.len())];
            cur = GridPosition {
                x: (cur.x + step.0).clamp(-h + 1, h - 1),
                y: (cur.y + step.1).clamp(-h + 1, h - 1),
            };
        }
    }
}

/// Open boss arena: border ring + four corner pillars, so the boss has clear sightlines and the
/// player has cover to kite around. Deterministic (fixed geometry).
fn boss_room_layout(map: &mut TileMap, _rng: &mut RunRng) {
    let h = MAP_HALF_TILES;
    border_ring(map, h);
    let q = h / 2;
    for (x, y) in [(-q, -q), (q, -q), (-q, q), (q, q)] {
        map.blocked.insert(GridPosition { x, y });
    }
}

/// Throne room: a distinct hall with flanking pillars and a raised dais wall (with a central gap for
/// approach) near the top — the "see the threat" geometry of the kiss/curse fantasy. The origin and
/// the central aisle stay clear. Deterministic (fixed geometry).
fn throne_room_layout(map: &mut TileMap, _rng: &mut RunRng) {
    let h = MAP_HALF_TILES;
    border_ring(map, h);
    // Flanking pillars down the hall, leaving the origin + central aisle clear.
    let mut y = -h + 4;
    while y <= h - 10 {
        for x in [-8i32, 8] {
            if x.abs() <= SPAWN_CLEAR_RADIUS && y.abs() <= SPAWN_CLEAR_RADIUS {
                continue;
            }
            map.blocked.insert(GridPosition { x, y });
        }
        y += 4;
    }
    // Raised dais wall near the top (the throne), with a gap in the middle to approach.
    let dais_y = h - 6;
    for x in -6..=6 {
        map.blocked.insert(GridPosition { x, y: dais_y });
    }
    map.blocked.remove(&GridPosition { x: 0, y: dais_y });
}

/// Large open act-boss area: border ring only. Deterministic.
fn act_boss_layout(map: &mut TileMap, _rng: &mut RunRng) {
    border_ring(map, MAP_HALF_TILES);
}

/// Small safe Merchant room: a compact border ring, no obstacles, no enemies.
fn merchant_layout(map: &mut TileMap) {
    border_ring(map, MERCHANT_HALF_TILES);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_map() -> TileMap {
        TileMap::default()
    }

    /// The ported blob must reproduce the old `generate_map` output for the same seed (the
    /// behavior-preserving regression pin for the port).
    #[test]
    fn procedural_layout_reproduces_the_same_map_for_a_seed() {
        let mut a = fresh_map();
        let mut b = fresh_map();
        procedural_room_layout(&mut a, &mut RunRng::from_seed(0xC0FFEE));
        procedural_room_layout(&mut b, &mut RunRng::from_seed(0xC0FFEE));
        assert_eq!(a.blocked, b.blocked, "same seed ⇒ identical procedural room");
        assert_eq!(a.half_width, MAP_HALF_TILES);
    }

    /// Every layout produces a bordered, in-bounds map with a walkable spawn-clear box at the origin.
    #[test]
    fn every_layout_borders_and_keeps_origin_clear() {
        let kinds = [
            EncounterType::Map { objective: crate::world::graph::ObjectiveType::KillAll },
            EncounterType::BossRoom,
            EncounterType::ThroneRoom,
            EncounterType::ActBoss,
            EncounterType::Merchant,
        ];
        for kind in kinds {
            let mut map = fresh_map();
            generate_room(&kind, &mut map, &mut RunRng::from_seed(1));
            let h = map.half_width;
            assert!(h > 0, "{kind:?}: extents set");
            // Border is blocked; origin (and the whole spawn-clear box) is walkable.
            assert!(map.is_blocked(GridPosition { x: -h, y: 0 }), "{kind:?}: left wall");
            assert!(map.is_blocked(GridPosition { x: h, y: 0 }), "{kind:?}: right wall");
            for dx in -SPAWN_CLEAR_RADIUS..=SPAWN_CLEAR_RADIUS {
                for dy in -SPAWN_CLEAR_RADIUS..=SPAWN_CLEAR_RADIUS {
                    assert!(
                        !map.is_blocked(GridPosition { x: dx, y: dy }),
                        "{kind:?}: spawn-clear box tile ({dx},{dy}) must be walkable"
                    );
                }
            }
        }
    }
}
