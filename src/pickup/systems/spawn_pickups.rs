use bevy::math::Vec2;
use bevy::prelude::{Commands, Query, Res, ResMut, With};
use bevy::time::Time;
use rand::Rng;
use crate::core::components::WorldPosition;
use crate::pickup::components::{PickUp, PickUpKind, PickUpSpawner};
use crate::pickup::constants::{
    HEAL_PACK_AMOUNT, PICKUP_SPAWN_MAX_DIST, PICKUP_SPAWN_MIN_DIST,
};
use crate::player::components::Player;

/// Builds a pickup entity at `pos` (logic components only). Shared by the timed spawner and
/// enemy death-drops. Visuals are attached by the presentation layer
/// (pickup/systems/visuals.rs), keyed off the PickUpKind.
pub fn spawn_pickup(commands: &mut Commands, pos: Vec2, kind: PickUpKind) {
    commands.spawn((
        PickUp { kind },
        WorldPosition(pos),
    ));
}

/// Periodically drops a healing pack on a ring around the player.
pub fn spawn_pickups_over_time(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<PickUpSpawner>,
    player: Query<&WorldPosition, With<Player>>,
) {
    spawner.timer.tick(time.delta());
    if !spawner.timer.finished() {
        return;
    }

    let Ok(player_pos) = player.single() else {
        return;
    };

    let mut rng = rand::thread_rng();
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let dist = rng.gen_range(PICKUP_SPAWN_MIN_DIST..PICKUP_SPAWN_MAX_DIST);
    let offset = Vec2::new(angle.cos(), angle.sin()) * dist;

    spawn_pickup(&mut commands, player_pos.0 + offset, PickUpKind::Heal(HEAL_PACK_AMOUNT));
}
