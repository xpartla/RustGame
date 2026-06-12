use bevy::prelude::Color;

// Map size. The map is a square of `MAP_HALF_TILES` tiles in each direction from the origin,
// i.e. (2*MAP_HALF_TILES + 1) tiles per side. Finite and bounded for the PoC — see the
// deferred "dynamic generation" note in PLAN.md / CLAUDE.md for streaming a larger world.
pub const MAP_HALF_TILES: i32 = 40;

// Keep a clear box of this Chebyshev radius (in tiles) around the origin so the player spawn
// and the inner enemy/pickup spawn rings are never walled in.
pub const SPAWN_CLEAR_RADIUS: i32 = 6;

// Scattered obstacle blobs: how many to grow, and the random-walk length (≈ tile count) of each.
pub const OBSTACLE_BLOB_COUNT: usize = 70;
pub const OBSTACLE_BLOB_MIN_TILES: u32 = 2;
pub const OBSTACLE_BLOB_MAX_TILES: u32 = 7;

// Rendering z-order: floor at 0 (replaces the old camera backdrop), obstacles just above it,
// below pickups (0.5), enemies (1) and the player (2).
pub const OBSTACLE_Z: f32 = 0.25;

pub const FLOOR_COLOR: Color = Color::srgb(0.18, 0.18, 0.26);
pub const OBSTACLE_COLOR: Color = Color::srgb(0.42, 0.39, 0.35);
