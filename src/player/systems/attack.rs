use bevy::input::ButtonInput;
use bevy::prelude::{Commands, Entity, KeyCode, Query, Res, Vec2, With};
use crate::core::components::WorldPosition;
use crate::player::components::{Facing, Player};
use crate::projectile::components::{ArcHitbox, CircleHitbox, ProjectileBundle};
use crate::constants::ARC_BASE_DMG;
use crate::constants::CIRCLE_BASE_DMG;
pub fn player_circle_attack(
    mut commands: Commands,
    kb: Res<ButtonInput<KeyCode>>,
    player: Query<(Entity, &WorldPosition, &Facing), With<Player>>
) {
    if !kb.just_pressed(KeyCode::Space) {
        return;
    }

    let(player_entity, pos, facing) = match player.get_single() {
        Ok(v) => v,
        Err(_) => return,
    };

    let attack_distance = 16.0;
    let spawn_pos = pos.0 + facing.0 * attack_distance;

    commands.spawn((
        ProjectileBundle::new(
            spawn_pos,
            Vec2::ZERO,
            CIRCLE_BASE_DMG,
            0.1,
            player_entity
        ),
        CircleHitbox {
            radius: 20.0,
        }
    ));
}

pub fn player_arc_attack(
    mut commands: Commands,
    kb: Res<ButtonInput<KeyCode>>,
    player: Query<(Entity, &WorldPosition, &Facing), With<Player>>
) {
    if !kb.just_pressed(KeyCode::KeyV) {
        return;
    }

    let(player_entity, pos, facing) = match player.get_single() {
        Ok(v) => v,
        Err(_) => return,
    };

    let (_, pos, facing) = player.get_single().unwrap();

    let attack_distance = 16.0;
    let spawn_pos = pos.0 + facing.0 * attack_distance;

    commands.spawn((
        ProjectileBundle::new(
            spawn_pos,
            Vec2::ZERO,
            ARC_BASE_DMG,
            0.1,
            player_entity
        ),
        ArcHitbox {
            radius: 20.0,
            half_angle: std::f32::consts::FRAC_PI_4,
        },
        Facing(facing.0),
    ));
}