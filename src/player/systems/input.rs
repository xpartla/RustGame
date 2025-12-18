use bevy::math::Vec2;
use bevy::prelude::{ButtonInput, KeyCode, Query, Res, With};
use crate::constants::PLAYER_SPEED;
use crate::player::components::Player;
use crate::core::components::Velocity;

pub fn player_input(
    kb: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    let mut vel = query.single_mut().unwrap();

    let mut dir = Vec2::ZERO;
    if kb.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if kb.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if kb.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if kb.pressed(KeyCode::KeyD) { dir.x += 1.0; }

    vel.0 = dir.normalize_or_zero() * PLAYER_SPEED;
}
