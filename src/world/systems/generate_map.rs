use bevy::prelude::ResMut;
use rand::Rng;
use crate::core::components::GridPosition;
use crate::run::rng::RunRng;
use crate::world::components::TileMap;
use crate::world::constants::{
    MAP_HALF_TILES, OBSTACLE_BLOB_COUNT, OBSTACLE_BLOB_MAX_TILES, OBSTACLE_BLOB_MIN_TILES,
    SPAWN_CLEAR_RADIUS,
};

/// 4-directional steps used to grow obstacle blobs via a short random walk.
const STEPS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Procedurally fills the `TileMap` (Startup): a solid border ring plus scattered obstacle
/// blobs grown by random walk. The spawn-clear box around the origin is always left walkable.
///
/// This is the deliberately simple PoC pass (scattered blobs on a finite grid). Rooms/corridors
/// and dynamic streaming beyond the fixed bounds are deferred — see PLAN.md / CLAUDE.md.
///
/// Draws from `RunRng` (the seeded run stream) rather than `thread_rng`, so the layout is
/// reproducible from the run seed. Phase 0 seeds `RunRng` from entropy per launch, so behaviour
/// is unchanged for now; Phase 7 supplies the real per-run seed.
pub fn generate_map(mut map: ResMut<TileMap>, mut run_rng: ResMut<RunRng>) {
    let h = MAP_HALF_TILES;
    map.half_width = h;
    map.half_height = h;
    map.blocked.clear();

    let rng = run_rng.rng();

    // Border ring — rendered as the boundary wall (out-of-bounds is impassable regardless).
    for x in -h..=h {
        map.blocked.insert(GridPosition { x, y: -h });
        map.blocked.insert(GridPosition { x, y: h });
    }
    for y in -h..=h {
        map.blocked.insert(GridPosition { x: -h, y });
        map.blocked.insert(GridPosition { x: h, y });
    }

    // Scattered obstacle blobs. Each starts at a random interior tile and random-walks a few
    // steps, blocking each tile it visits (skipping the spawn-clear box around the origin).
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
