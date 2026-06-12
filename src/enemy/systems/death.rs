use bevy::asset::Assets;
use bevy::prelude::{Commands, Entity, Mesh, Query, ResMut, With};
use bevy::sprite::ColorMaterial;
use rand::Rng;
use crate::core::components::{Health, WorldPosition};
use crate::enemy::components::Enemy;
use crate::pickup::components::PickUpKind;
use crate::pickup::constants::{ENEMY_DROP_CHANCE, HEAL_PACK_AMOUNT};
use crate::pickup::spawn_pickup;

pub fn enemy_death(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    query: Query<(Entity, &WorldPosition, &Health), With<Enemy>>,
) {
    let mut rng = rand::thread_rng();
    for (entity, pos, health) in &query {
        if health.current <= 0.0 {
            // Chance to drop a healing pack where the enemy fell.
            if rng.gen_range(0.0..1.0) < ENEMY_DROP_CHANCE {
                spawn_pickup(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    pos.0,
                    PickUpKind::Heal(HEAL_PACK_AMOUNT),
                );
            }
            commands.entity(entity).despawn();
        }
    }
}