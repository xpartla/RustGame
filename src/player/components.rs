use bevy::math::Vec2;
use bevy::prelude::Component;

#[derive(Component)]
pub struct Player;

#[derive(Component, Debug, Copy, Clone)]
pub struct Facing(pub Vec2);