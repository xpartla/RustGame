use bevy::prelude::Query;
use crate::core::components::{GridPosition, WorldPosition};


pub fn world_to_grid(
    mut query: Query<(&WorldPosition, &mut GridPosition)>,
) {
    for (world, mut grid) in &mut query {
        *grid = GridPosition::from_world(world.0);
    }
}
