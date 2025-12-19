use bevy::prelude::{Gizmos, Query, Vec2, With};
use crate::core::components::{GridPosition, WorldPosition};
use crate::enemy::components::Enemy;
use bevy::color::palettes::css::{ORANGE, RED};
use crate::constants::TILE_SIZE;

pub fn draw_enemy_grid_positions(
    enemies: Query<&GridPosition, With<Enemy>>,
    mut gizmos: Gizmos,
) {
    for pos in &enemies {
        let world = Vec2::new(
            pos.x as f32 * TILE_SIZE,
            pos.y as f32 * TILE_SIZE,
        );

        gizmos
            .rect_2d(
                world,
                Vec2::splat(TILE_SIZE),
                RED,
            )

    }
}

pub fn draw_enemy_world_positions(
    enemies: Query<&WorldPosition, With<Enemy>>,
    mut gizmos: Gizmos,
) {
    for pos in &enemies {
        gizmos.circle_2d(pos.0, 10.0, ORANGE);
    }
}
