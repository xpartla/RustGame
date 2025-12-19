use bevy::prelude::{Component, Resource, Timer};

#[derive(Component)]
pub struct Enemy;

#[derive(Resource)]
pub struct EnemySpawner {
    pub timer: Timer,
    pub radius: i32,
}