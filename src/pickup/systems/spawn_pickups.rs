use bevy::asset::Assets;
use bevy::color::Color;
use bevy::math::Vec2;
use bevy::prelude::{
    Circle, Commands, Mesh, Mesh2d, MeshMaterial2d, Query, Res, ResMut, Transform, With,
};
use bevy::sprite::ColorMaterial;
use bevy::time::Time;
use rand::Rng;
use crate::core::components::WorldPosition;
use crate::pickup::components::{PickUp, PickUpKind, PickUpSpawner};
use crate::pickup::constants::{
    HEAL_PACK_AMOUNT, HEAL_PACK_VISUAL_RADIUS, PICKUP_SPAWN_MAX_DIST, PICKUP_SPAWN_MIN_DIST,
};
use crate::player::components::Player;

/// Builds a pickup entity at `pos`. Shared by the timed spawner and enemy death-drops so the
/// mesh is constructed in exactly one place. A healing pack renders as a small green circle at
/// z=0.5 (below enemies at z=1 and the player at z=2). `sync_transform` keeps its x/y.
pub fn spawn_pickup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    pos: Vec2,
    kind: PickUpKind,
) {
    let (mesh_radius, color) = match kind {
        PickUpKind::Heal(_) => (HEAL_PACK_VISUAL_RADIUS, Color::srgb(0.1, 0.9, 0.2)),
    };

    commands.spawn((
        PickUp { kind },
        WorldPosition(pos),
        Transform::from_xyz(pos.x, pos.y, 0.5),
        Mesh2d(meshes.add(Circle::new(mesh_radius))),
        MeshMaterial2d(materials.add(color)),
    ));
}

/// Periodically drops a healing pack on a ring around the player.
pub fn spawn_pickups_over_time(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<PickUpSpawner>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
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

    spawn_pickup(
        &mut commands,
        &mut meshes,
        &mut materials,
        player_pos.0 + offset,
        PickUpKind::Heal(HEAL_PACK_AMOUNT),
    );
}
