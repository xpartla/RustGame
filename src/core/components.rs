use std::collections::HashMap;
use bevy::math::{IVec2, Vec2};
use bevy::prelude::{Component, Resource};

#[derive(Component, Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Component, Debug, Copy, Clone)]
pub struct WorldPosition(pub Vec2);

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self {
            current: max,
            max,
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct FlowField {
    pub cost: HashMap<GridPosition, u32>,
    pub direction: HashMap<GridPosition, Vec2>,
}