use bevy::prelude::{Commands, Entity, Query, With};
use crate::core::components::WorldPosition;
use crate::enemy::components::Enemy;
use crate::projectile::components::{Hitbox, Projectile, Source};

pub fn projectile_hits_enemies(
    mut commands: Commands,
    projectiles: Query<(Entity, &WorldPosition, &Hitbox, &Source), With<Projectile>>,
    enemies: Query<(Entity, &WorldPosition), With<Enemy>>,
) {
    for(proj_entity, proj_pos, hitbox, source) in &projectiles {
        for(enemy_entity, enemy_pos) in &enemies {
            if enemy_entity == source.entity {
                continue;
            }
            let dist = proj_pos.0.distance(enemy_pos.0);
            if dist <= hitbox.radius {
                commands.entity(enemy_entity).despawn();
                commands.entity(proj_entity).despawn();
                break;
            }
        }
    }
}