use std::f32::consts::FRAC_PI_2;
use bevy::prelude::{Quat, Query, Transform};
use crate::core::components::{Facing, WorldPosition};

pub fn sync_transform(
    mut query: Query<(&WorldPosition, &mut Transform)>,
) {
    for (pos, mut transform) in &mut query {
        transform.translation.x = pos.0.x;
        transform.translation.y = pos.0.y;
    }
}

/// Writes each entity's `Facing` into its `Transform.rotation` so meshes visually point where
/// they're aimed. The `-FRAC_PI_2` offset accounts for Bevy's 2D primitives (`RegularPolygon`,
/// `Rectangle`) defaulting to point along +Y; subtracting a quarter-turn maps a `Facing` of +X
/// to "pointing right". Visible on the triangle/square enemies; circles (player, grunts) rotate
/// invisibly. Skips near-zero facings to avoid `atan2(0, 0)` noise.
pub fn apply_facing_rotation(
    mut query: Query<(&Facing, &mut Transform)>,
) {
    for (facing, mut transform) in &mut query {
        if facing.0.length_squared() > 0.0001 {
            let angle = facing.0.y.atan2(facing.0.x) - FRAC_PI_2;
            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}
