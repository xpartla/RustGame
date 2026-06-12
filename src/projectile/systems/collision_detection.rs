use bevy::prelude::{Commands, Entity, EventWriter, Query, With};
use crate::core::components::WorldPosition;
use crate::core::events::DamageEvent;
use crate::enemy::components::Enemy;
use crate::player::components::Facing;
use crate::projectile::components::{ArcHitbox, CircleHitbox, Damage, Projectile, Source};

pub fn projectile_circle_hits_enemies(
    mut commands: Commands,
    mut damage_events: EventWriter<DamageEvent>,
    projectiles: Query<(Entity, &WorldPosition, &CircleHitbox, &Source, &Damage), With<Projectile>>,
    enemies: Query<(Entity, &WorldPosition), With<Enemy>>,
) {
    for(proj_entity, proj_pos, hitbox, source, damage) in &projectiles {
        for(enemy_entity, enemy_pos) in &enemies {
            if enemy_entity == source.entity {
                continue;
            }
            let dist = proj_pos.0.distance(enemy_pos.0);
            if dist <= hitbox.radius {
                damage_events.write(DamageEvent {
                    target: enemy_entity,
                    amount: damage.0,
                    source: source.entity,
                });
                commands.entity(proj_entity).despawn();
                break;
            }
        }
    }
}

pub fn projectile_arc_hit_enemies(
    mut commands: Commands,
    mut damage_events: EventWriter<DamageEvent>,
    projectiles: Query<(Entity, &WorldPosition, &ArcHitbox, &Facing, &Source, &Damage), With<Projectile>>,
    enemies: Query<(Entity, &WorldPosition), With<Enemy>>,
) {
    for (proj_entity, proj_pos, arc, facing, source, damage) in &projectiles {
        let forward = facing.0.normalize();

        for (enemy_entity, enemy_pos) in &enemies {
            if enemy_entity == source.entity {
                continue;
            }

            let to_enemy = enemy_pos.0 - proj_pos.0;
            let dist = to_enemy.length();

            if dist > arc.radius {
                continue;
            }

            let dir = to_enemy.normalize();
            let angle = forward.angle_to(dir).abs();

            if  angle <= arc.half_angle {
                damage_events.write(DamageEvent {
                    target: enemy_entity,
                    amount: damage.0,
                    source: source.entity,
                });
                commands.entity(proj_entity).despawn();
                break;
            }
        }

    }
}
