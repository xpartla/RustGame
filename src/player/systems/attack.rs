use bevy::input::ButtonInput;
use bevy::prelude::{Commands, Entity, KeyCode, Query, Res, Vec2, With};
use crate::core::components::WorldPosition;
use crate::player::components::Player;
use crate::projectile::components::ProjectileBundle;
pub fn player_melee_attack(
    mut commands: Commands,
    kb: Res<ButtonInput<KeyCode>>,
    player: Query<(Entity, &WorldPosition), With<Player>>
) {
    if !kb.just_pressed(KeyCode::Space) {
        return;
    }
    
    let(player_entity, pos) = match player.get_single() {
        Ok(v) => v,
        Err(_) => return,
    };
    
    let forward = Vec2::X;
    let offset = forward * 16.0;
    
    commands.spawn(ProjectileBundle::new(
        pos.0 + offset,
        forward * 0.0,
        1,
        20.0,
        0.1,
        player_entity,
    ));
}