use bevy::prelude::{Query, With};
use crate::core::components::{Facing, Velocity};
use crate::enemy::components::{AiBehavior, Enemy};

/// Orients each melee-chaser enemy toward its current movement direction (the flow-field-driven
/// velocity). Runs after `enemy_follow_flow_field` so it reads the freshly-lerped velocity. Skips
/// near-zero velocities to avoid normalizing a zero vector (NaN) and to keep the last facing while
/// momentarily stationary. Ranged/stationary AI aim at the player instead (their own systems).
pub fn update_enemy_facing(
    mut enemies: Query<(&Velocity, &mut Facing, &AiBehavior), With<Enemy>>,
) {
    for (vel, mut facing, ai) in &mut enemies {
        if !matches!(ai, AiBehavior::MeleeChaser) {
            continue;
        }
        if vel.0.length_squared() > 0.0001 {
            facing.0 = vel.0.normalize();
        }
    }
}
