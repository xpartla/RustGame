use bevy::prelude::{Query, Res, Vec2, With};
use crate::core::components::{FlowField, GridPosition, Velocity};
use crate::enemy::components::{AiBehavior, Enemy, MoveSpeed};

pub fn enemy_follow_flow_field(
    flow_field: Res<FlowField>,
    mut enemies: Query<(&GridPosition, &mut Velocity, &MoveSpeed, &AiBehavior), With<Enemy>>,
) {
    for (grid_pos, mut vel, speed, ai) in &mut enemies {
        // Only flow-field chasers steer here; ranged/stationary AI drive their own movement.
        if !matches!(ai, AiBehavior::MeleeChaser) {
            continue;
        }
        if let Some(direction) = flow_field.direction.get(grid_pos) {
            let desired = direction.normalize_or_zero() * speed.0;
            vel.0 = vel.0.lerp(desired, 0.2);
        } else {
            vel.0 = Vec2::ZERO;
        }
    }
}
