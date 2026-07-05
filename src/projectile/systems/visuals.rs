// Presentation-layer dress-up for travelling projectiles (Phase 4). Registered only by
// PresentationPlugin — headless simulations skip it, so it never touches the golden baseline.
//
// Before Phase 4 projectiles (Fireblast, Frostbolt, …) were logic-only entities with no Transform
// or mesh — invisible on the Windows build. This attaches a small circle mesh on spawn, tinted by
// the projectile's elemental damage tag. sync_transform (also presentation) then follows the
// projectile's WorldPosition each frame.

use bevy::asset::Assets;
use bevy::color::Color;
use bevy::prelude::{
    Added, Circle, ColorMaterial, Commands, Entity, Mesh, Mesh2d, MeshMaterial2d, Query, ResMut,
    Transform,
};
use crate::ability::effects::ResolvedEffect;
use crate::core::components::WorldPosition;
use crate::core::events::DamageTag;
use crate::projectile::components::{ProjectileMotion, ProjectilePayload};

/// Attaches Transform + mesh + material to a freshly spawned travelling projectile.
/// z=1.5: above enemies (z=1), below the player (z=2). sync_transform keeps x/y.
pub fn attach_projectile_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    projectiles: Query<
        (Entity, &WorldPosition, &ProjectileMotion, Option<&ProjectilePayload>),
        Added<ProjectileMotion>,
    >,
) {
    for (entity, pos, motion, payload) in &projectiles {
        let color = payload
            .and_then(projectile_color)
            .unwrap_or(Color::srgb(0.95, 0.95, 0.7)); // default: pale bolt
        commands.entity(entity).insert((
            Transform::from_xyz(pos.0.x, pos.0.y, 1.5),
            Mesh2d(meshes.add(Circle::new(motion.radius.max(3.0)))),
            MeshMaterial2d(materials.add(color)),
        ));
    }
}

/// Picks a projectile tint from the first elemental Damage effect it carries (physical / effectless
/// projectiles fall back to the default bolt color).
fn projectile_color(payload: &ProjectilePayload) -> Option<Color> {
    payload.effects.iter().find_map(|e| match e {
        ResolvedEffect::Damage { tags, .. } => tags.iter().find_map(|t| match t {
            DamageTag::Fire => Some(Color::srgb(1.0, 0.45, 0.1)),
            DamageTag::Frost => Some(Color::srgb(0.4, 0.7, 1.0)),
            DamageTag::Holy => Some(Color::srgb(1.0, 0.95, 0.5)),
            DamageTag::Shadow => Some(Color::srgb(0.5, 0.2, 0.6)),
            DamageTag::Arcane => Some(Color::srgb(0.7, 0.4, 1.0)),
            DamageTag::Physical => None,
        }),
        _ => None,
    })
}
