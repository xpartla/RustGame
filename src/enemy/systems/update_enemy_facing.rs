use bevy::prelude::{Query, With};
use crate::core::components::{Facing, Velocity};
use crate::enemy::components::Enemy;

/// Orients each enemy toward its current movement direction (the flow-field-driven velocity).
/// Runs after `enemy_follow_flow_field` so it reads the freshly-lerped velocity. Skips
/// near-zero velocities to avoid normalizing a zero vector (NaN) and to keep the last facing
/// while momentarily stationary.
pub fn update_enemy_facing(
    mut enemies: Query<(&Velocity, &mut Facing), With<Enemy>>,
) {
    for (vel, mut facing) in &mut enemies {
        if vel.0.length_squared() > 0.0001 {
            facing.0 = vel.0.normalize();
        }
    }
}
