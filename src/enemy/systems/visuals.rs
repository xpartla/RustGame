// Presentation-layer dress-up for enemy entities. Registered only by PresentationPlugin —
// headless simulations skip it. The mesh shape/size/color come from the EnemyAppearance data
// the logic spawn copied off the archetype; the construction here is exactly what the old
// spawner built inline.

use bevy::asset::Assets;
use bevy::prelude::{
    Added, Circle, ColorMaterial, Commands, Entity, Mesh, Mesh2d, MeshMaterial2d, Query,
    Rectangle, RegularPolygon, ResMut, Transform,
};
use crate::core::components::WorldPosition;
use crate::enemy::archetypes::EnemyShape;
use crate::enemy::components::{Enemy, EnemyAppearance};

/// Attaches Transform + mesh + material to a freshly spawned enemy.
/// z=1: above the background (z=0), below the player (z=2). sync_transform keeps x/y.
pub fn attach_enemy_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    enemies: Query<(Entity, &WorldPosition, &EnemyAppearance), Added<Enemy>>,
) {
    for (entity, pos, appearance) in &enemies {
        let mesh = match appearance.shape {
            EnemyShape::Circle => meshes.add(Circle::new(appearance.radius)),
            EnemyShape::Square => {
                meshes.add(Rectangle::new(appearance.radius * 2.0, appearance.radius * 2.0))
            }
            EnemyShape::Triangle => meshes.add(RegularPolygon::new(appearance.radius, 3)),
        };
        commands.entity(entity).insert((
            Transform::from_xyz(pos.0.x, pos.0.y, 1.0),
            Mesh2d(mesh),
            MeshMaterial2d(materials.add(appearance.color)),
        ));
    }
}
