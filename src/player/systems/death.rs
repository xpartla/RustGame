use bevy::log::info;
use bevy::prelude::{Commands, Entity, Query, With};
use crate::core::components::Health;
use crate::player::components::Player;

/// Minimal player-death placeholder for the PoC: log it and despawn the player so the
/// run visibly ends. Player-dependent systems (input, camera follow) are written to no-op
/// when no player exists.
// TODO(Phase: menu/UI epic): replace with a real game-over state (GameState::GameOver),
// death screen, and restart flow.
pub fn player_death(
    mut commands: Commands,
    query: Query<(Entity, &Health), With<Player>>,
) {
    for (entity, health) in &query {
        if health.current <= 0.0 {
            info!("Player died.");
            commands.entity(entity).despawn();
        }
    }
}
