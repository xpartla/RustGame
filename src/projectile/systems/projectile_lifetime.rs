use bevy::prelude::{Commands, Entity, Query, Res, Time, With};
use crate::projectile::components::{Lifetime, Projectile};

pub fn projectile_lifetime(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Lifetime), With<Projectile>>,
) {
    for (entity, mut lifetime) in &mut query {
        lifetime.timer.tick(time.delta());
        if(lifetime.timer.finished()){
            commands.entity(entity).despawn();
        }
    }
}