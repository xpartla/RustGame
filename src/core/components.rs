use bevy::math::Vec2;
use bevy::prelude::Component;

#[derive(Component, Debug, Copy, Clone)]
pub struct  GridPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Component, Debug, Copy, Clone)]
pub struct WorldPosition(pub Vec2);

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);