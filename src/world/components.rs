use std::collections::HashSet;
use bevy::prelude::Resource;
use crate::core::components::GridPosition;

/// The procedurally generated tile map. Authoritative record of which tiles are walkable.
///
/// Representation is **sparse**: the map is a finite square of half-extents
/// `half_width`/`half_height` (in tiles, centered on the origin), and only *blocked* tiles are
/// stored in `blocked`. Anything outside the bounds is treated as impassable too, so the map
/// edge acts as an invisible wall. Keys are `GridPosition`, so the map drops straight into the
/// flow field and the movement collision check.
#[derive(Resource, Default)]
pub struct TileMap {
    pub half_width: i32,
    pub half_height: i32,
    pub blocked: HashSet<GridPosition>,
}

impl TileMap {
    /// True if `pos` is inside the finite map bounds.
    pub fn in_bounds(&self, pos: GridPosition) -> bool {
        pos.x >= -self.half_width
            && pos.x <= self.half_width
            && pos.y >= -self.half_height
            && pos.y <= self.half_height
    }

    /// True if an entity may **not** occupy `pos` — either it is out of bounds or an obstacle.
    pub fn is_blocked(&self, pos: GridPosition) -> bool {
        !self.in_bounds(pos) || self.blocked.contains(&pos)
    }
}
