// Presentation-layer dress-up for pickup entities. Registered only by PresentationPlugin —
// headless simulations skip it. A healing pack renders as a small green circle at z=0.5
// (below enemies at z=1 and the player at z=2), exactly as the old spawn_pickup built inline.

use bevy::asset::Assets;
use bevy::color::Color;
use bevy::prelude::{Added, Circle, ColorMaterial, Commands, Entity, Mesh, Mesh2d, MeshMaterial2d, Query, ResMut, Transform};
use crate::core::components::WorldPosition;
use crate::pickup::components::{PickUp, PickUpKind};
use crate::pickup::constants::HEAL_PACK_VISUAL_RADIUS;

/// Attaches Transform + mesh + material to a freshly spawned pickup.
pub fn attach_pickup_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    pickups: Query<(Entity, &WorldPosition, &PickUp), Added<PickUp>>,
) {
    for (entity, pos, pickup) in &pickups {
        let (mesh_radius, color) = match pickup.kind {
            PickUpKind::Heal(_) => (HEAL_PACK_VISUAL_RADIUS, Color::srgb(0.1, 0.9, 0.2)),
            // Bloom flower (Phase 9.4) — a distinct pink/violet from the green heal pack.
            PickUpKind::Enhance(_) => (HEAL_PACK_VISUAL_RADIUS, Color::srgb(0.85, 0.4, 0.85)),
        };
        commands.entity(entity).insert((
            Transform::from_xyz(pos.0.x, pos.0.y, 0.5),
            Mesh2d(meshes.add(Circle::new(mesh_radius))),
            MeshMaterial2d(materials.add(color)),
        ));
    }
}
