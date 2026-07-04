use std::time::Duration;
use bevy::color::Color;
use bevy::prelude::{Component, Resource, Timer, TimerMode};
use crate::enemy::archetypes::EnemyShape;

#[derive(Component)]
pub struct Enemy;

/// Visual identity copied from the archetype at spawn. Pure data — the presentation layer
/// (enemy/systems/visuals.rs) reads it to build the Mesh2d/material, so headless simulations
/// never touch render assets.
#[derive(Component, Clone, Copy)]
pub struct EnemyAppearance {
    pub shape: EnemyShape,
    pub radius: f32,
    pub color: Color,
}

#[derive(Resource)]
pub struct EnemySpawner {
    pub timer: Timer,
    pub radius: i32,
}

/// Per-entity movement speed (world units/sec). Set from the enemy's archetype at spawn.
#[derive(Component)]
pub struct MoveSpeed(pub f32);

/// Per-entity contact-attack stats. Set from the enemy's archetype at spawn.
#[derive(Component)]
pub struct AttackStats {
    pub damage: f32,
    pub range: f32,
}

/// Experience awarded to the killer when this enemy dies. Set from the archetype at spawn.
#[derive(Component)]
pub struct XpReward(pub u32);

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