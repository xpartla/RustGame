use bevy::math::Vec2;
use bevy::prelude::{Commands, Transform};
use crate::core::components::{GridPosition, Velocity, WorldPosition};
use crate::enemy::components::Enemy;

pub fn spawn_enemy(
    mut commands: Commands,
) {
    commands.spawn((
        Enemy,
        GridPosition { x: 5, y: -3 },
        WorldPosition(Vec2::ZERO),
        Velocity::default(),
        Transform::default(),
    ));
}
