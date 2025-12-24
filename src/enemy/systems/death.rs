use bevy::prelude::{Commands, Entity, Query, With};
use crate::enemy::components::{Enemy, Health};

pub fn enemy_death(
    mut commands: Commands,
    query: Query<(Entity, &Health), With<Enemy>>,
) {
    for (entity, health) in &query {
        if health.current <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}