use std::time::Duration;
use bevy::prelude::{Component, Resource, Timer, TimerMode};

#[derive(Component)]
pub struct Enemy;

#[derive(Resource)]
pub struct EnemySpawner {
    pub timer: Timer,
    pub radius: i32,
}

/// Gates how often an enemy can deal contact damage to the player.
#[derive(Component)]
pub struct AttackCooldown {
    pub timer: Timer,
}

impl AttackCooldown {
    /// Starts *ready* so the first time an enemy reaches the player it hits immediately,
    /// then once every `seconds`.
    pub fn new(seconds: f32) -> Self {
        let mut timer = Timer::from_seconds(seconds, TimerMode::Once);
        timer.tick(Duration::from_secs_f32(seconds));
        Self { timer }
    }
}