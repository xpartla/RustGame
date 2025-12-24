use bevy::prelude::{Component, Resource, Timer};

#[derive(Component)]
pub struct Enemy;

#[derive(Resource)]
pub struct EnemySpawner {
    pub timer: Timer,
    pub radius: i32,
}

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