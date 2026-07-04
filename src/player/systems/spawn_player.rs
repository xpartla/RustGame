use bevy::math::Vec2;
use bevy::prelude::Commands;
use crate::constants::PLAYER_HEALTH;
use crate::core::components::{GridPosition, Health, Velocity, WorldPosition};
use crate::core::components::Facing;
use crate::player::components::{Experience, Player};

/// Spawns the player with logic components only. Visuals (Transform, Mesh2d, material) are
/// attached by the presentation layer (player/systems/visuals.rs) so headless simulations
/// never touch render assets.
pub fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player,
        Health::new(PLAYER_HEALTH),
        Experience::new(),
        WorldPosition(Vec2::ZERO),
        GridPosition{x:0, y:0},
        Facing(Vec2::default()),
        Velocity::default(),
    ));
}
