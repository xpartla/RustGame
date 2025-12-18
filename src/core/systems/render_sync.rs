use bevy::prelude::{Query, Transform};
use crate::core::components::WorldPosition;

pub fn sync_transform(
    mut query: Query<(&WorldPosition, &mut Transform)>,
) {
    for (pos, mut transform) in &mut query {
        transform.translation.x = pos.0.x;
        transform.translation.y = pos.0.y;
    }
}
