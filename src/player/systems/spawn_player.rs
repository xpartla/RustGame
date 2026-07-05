use bevy::math::Vec2;
use bevy::prelude::Commands;
use crate::constants::{PLAYER_HEALTH, PLAYER_RADIUS};
use crate::core::components::{GridPosition, Health, Hurtbox, Velocity, WorldPosition};
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
        // No projectile can hit the player until enemies shoot (Phase 5), but the hurtbox is
        // part of the actor's logic identity, so it spawns with the rest.
        Hurtbox { radius: PLAYER_RADIUS },
    ));
}
