use bevy::input::ButtonInput;
use bevy::prelude::{Commands, Entity, KeyCode, Query, Res, Vec2, With};
use crate::core::components::WorldPosition;
use crate::player::components::{Facing, Player};
use crate::projectile::components::ProjectileBundle;
pub fn player_melee_attack(
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

    commands.spawn(ProjectileBundle::new(
        spawn_pos,
        Vec2::ZERO,
        1,
        20.0,
        0.1,
        player_entity,
    ));
}