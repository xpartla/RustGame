use bevy::asset::Assets;
use bevy::prelude::{Commands, Entity, EventWriter, Mesh, Query, ResMut, With};
use bevy::sprite::ColorMaterial;
use rand::Rng;
use crate::core::components::{Health, LastHitBy, WorldPosition};
use crate::core::events::GainXpEvent;
use crate::enemy::components::{Enemy, XpReward};
use crate::pickup::components::PickUpKind;
use crate::pickup::constants::{ENEMY_DROP_CHANCE, HEAL_PACK_AMOUNT};
use crate::pickup::spawn_pickup;

pub fn enemy_death(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut xp_events: EventWriter<GainXpEvent>,
    query: Query<(Entity, &WorldPosition, &Health, &XpReward, &LastHitBy), With<Enemy>>,
) {
    let mut rng = rand::thread_rng();
    for (entity, pos, health, xp, last_hit_by) in &query {
        if health.current <= 0.0 {
            // Award XP to whoever landed the killing blow (the player). A no-op if the killer
            // has no `Experience` component.
            xp_events.write(GainXpEvent { target: last_hit_by.0, amount: xp.0 });

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