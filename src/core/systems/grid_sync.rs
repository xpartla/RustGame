use bevy::prelude::Query;
use crate::constants::TILE_SIZE;
use crate::core::components::{GridPosition, WorldPosition};


pub fn world_to_grid(
    mut query: Query<(&WorldPosition, &mut GridPosition)>,
) {
    for (world, mut grid) in &mut query {
        grid.x = (world.0.x / TILE_SIZE).round() as i32;
        grid.y = (world.0.y / TILE_SIZE).round() as i32;
    }
}
