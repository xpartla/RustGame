use bevy::math::Vec2;
use bevy::prelude::{ButtonInput, KeyCode, Query, Res, With};
use crate::player::components::Player;
use crate::core::components::{MoveSpeed, Velocity};

pub fn player_input(
    kb: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Velocity, &MoveSpeed), With<Player>>,
) {
    let Ok((mut vel, speed)) = query.single_mut() else {
        return;
    };

    let mut dir = Vec2::ZERO;
    if kb.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if kb.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if kb.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if kb.pressed(KeyCode::KeyD) { dir.x += 1.0; }

    vel.0 = dir.normalize_or_zero() * speed.0;
}
