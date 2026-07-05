// Presentation-layer dress-up for the player entity. Registered only by PresentationPlugin —
// headless simulations skip it. Values (radius, color, z-layer) are exactly what the old
// spawn_player inserted inline.

use bevy::asset::Assets;
use bevy::color::Color;
use bevy::prelude::{Added, Circle, ColorMaterial, Commands, Entity, Mesh, Mesh2d, MeshMaterial2d, Query, ResMut, Transform};
use crate::constants::PLAYER_RADIUS;
use crate::core::components::WorldPosition;
use crate::player::components::Player;

/// Attaches Transform + mesh + material to a freshly spawned player.
/// z=2: above enemies (z=1) and pickups (z=0.5). sync_transform keeps x/y.
pub fn attach_player_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    players: Query<(Entity, &WorldPosition), Added<Player>>,
) {
    for (entity, pos) in &players {
        commands.entity(entity).insert((
            Transform::from_xyz(pos.0.x, pos.0.y, 2.),
            Mesh2d(meshes.add(Circle::new(PLAYER_RADIUS))),
            MeshMaterial2d(materials.add(Color::srgb(6.25, 9.4, 9.1))),
        ));
    }
}
