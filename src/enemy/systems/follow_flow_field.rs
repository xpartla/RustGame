use bevy::prelude::{IVec2, Query, Res, Vec2, With};
use crate::constants::{ENEMY_SPEED};
use crate::core::components::{FlowField, GridPosition, Velocity};
use crate::enemy::components::Enemy;

pub fn enemy_follow_flow_field(
    flow_field: Res<FlowField>,
    mut enemies: Query<(&GridPosition, &mut Velocity), With<Enemy>>,
) {
    for (grid_pos, mut vel) in &mut enemies {
        if let Some(direction) = flow_field.direction.get(grid_pos) {
            vel.0 = direction.normalize_or_zero() * ENEMY_SPEED;
        } else {
            vel.0 = Vec2::ZERO;
        }
    }
}
