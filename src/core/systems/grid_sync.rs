use bevy::prelude::{Added, Query, Vec2};
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

pub fn grid_to_world(
    mut query: Query<(&GridPosition, &mut WorldPosition), Added<GridPosition>>,
) {
    for (grid, mut world) in &mut query {
        world.0 = Vec2::new(
            grid.x as f32 * TILE_SIZE,
            grid.y as f32 * TILE_SIZE,
        );
    }
}
