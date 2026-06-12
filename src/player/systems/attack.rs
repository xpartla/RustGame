use bevy::input::ButtonInput;
use bevy::prelude::{Commands, Entity, KeyCode, Query, Res, Vec2, With};
use crate::core::components::WorldPosition;
use crate::player::components::{Facing, Player};
use crate::projectile::components::{ArcHitbox, CircleHitbox, ProjectileBundle};
use crate::constants::{
    ARC_BASE_DMG, CIRCLE_BASE_DMG, ATTACK_SPAWN_DISTANCE, ATTACK_HITBOX_RADIUS, ATTACK_LIFETIME,
};

pub fn player_circle_attack(
    mut commands: Commands,
    kb: Res<ButtonInput<KeyCode>>,
    player: Query<(Entity, &WorldPosition, &Facing), With<Player>>
) {
    if !kb.just_pressed(KeyCode::Space) {
        return;
    }

    let(player_entity, pos, facing) = match player.single() {
        Ok(v) => v,
        Err(_) => return,
    };

    // No aim direction yet (Facing starts at zero until the first mouse move).
    if facing.0.length_squared() < 1e-6 {
        return;
    }

    let spawn_pos = pos.0 + facing.0 * ATTACK_SPAWN_DISTANCE;

    commands.spawn((
        ProjectileBundle::new(
            spawn_pos,
            Vec2::ZERO,
            CIRCLE_BASE_DMG,
            ATTACK_LIFETIME,
            player_entity
        ),
        CircleHitbox {
            radius: ATTACK_HITBOX_RADIUS,
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

    let(player_entity, pos, facing) = match player.single() {
        Ok(v) => v,
        Err(_) => return,
    };

    // Arc collision normalizes facing; skip while there is no aim direction yet.
    if facing.0.length_squared() < 1e-6 {
        return;
    }

    let spawn_pos = pos.0 + facing.0 * ATTACK_SPAWN_DISTANCE;

    commands.spawn((
        ProjectileBundle::new(
            spawn_pos,
            Vec2::ZERO,
            ARC_BASE_DMG,
            ATTACK_LIFETIME,
            player_entity
        ),
        ArcHitbox {
            radius: ATTACK_HITBOX_RADIUS,
            half_angle: std::f32::consts::FRAC_PI_4,
        },
        Facing(facing.0),
    ));
}
