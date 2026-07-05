// Presentation dress-up for persistent zones (Phase 7.5F). Registered only by PresentationPlugin —
// headless simulations skip it, so it never touches the golden baseline (closes the phase-6
// zone-visuals deferral).
//
// Zones are logic entities carrying a `PersistentZone` (type + radius) and a `WorldPosition`. This
// attaches a translucent disc mesh on spawn, colored by zone type; `sync_transform` (also
// presentation) then keeps it under a FollowCaster zone as the anchor moves. The disc sits at z=0.4,
// below enemies (z=1) and the player (z=2), so it reads as ground.

use bevy::asset::Assets;
use bevy::color::Color;
use bevy::prelude::{
    Added, Circle, ColorMaterial, Commands, Entity, Mesh, Mesh2d, MeshMaterial2d, Query, ResMut,
    Transform,
};

use crate::core::components::WorldPosition;
use crate::zone::components::PersistentZone;

/// Attaches a translucent disc to a freshly spawned zone, tinted by its `zone_type`.
pub fn attach_zone_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    zones: Query<(Entity, &WorldPosition, &PersistentZone), Added<PersistentZone>>,
) {
    for (entity, pos, zone) in &zones {
        commands.entity(entity).insert((
            Transform::from_xyz(pos.0.x, pos.0.y, 0.4),
            Mesh2d(meshes.add(Circle::new(zone.radius.max(4.0)))),
            MeshMaterial2d(materials.add(zone_color(&zone.zone_type))),
        ));
    }
}

/// A translucent tint per zone type (falls back to a neutral grey for unknown types).
fn zone_color(zone_type: &str) -> Color {
    match zone_type {
        "death_and_decay" => Color::srgba(0.75, 0.1, 0.15, 0.28), // DK blood red
        "consecrated_ground" => Color::srgba(0.95, 0.85, 0.4, 0.28), // Paladin gold
        "amz" => Color::srgba(0.3, 0.5, 0.95, 0.25),               // anti-magic blue
        "tree_conduit" => Color::srgba(0.3, 0.8, 0.35, 0.25),      // Druid green
        _ => Color::srgba(0.6, 0.6, 0.6, 0.22),
    }
}
