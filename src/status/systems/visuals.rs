// Presentation-layer status tinting (Phase 4). Registered only by PresentationPlugin — headless
// simulations skip it, so it never touches the golden baseline.
//
// Recolors each enemy's material toward the color of its active status (frostbite blue, blaze
// orange, bleed red, root/stun yellow), reverting to the archetype color when no tinted status is
// active. Recomputed every frame so it self-corrects the moment an effect expires. Each enemy owns
// its own ColorMaterial (attach_enemy_visuals), so tinting one never affects another.

use bevy::color::Color;
use bevy::prelude::*;
use std::collections::HashMap;
use crate::enemy::components::{Enemy, EnemyAppearance};
use crate::status::components::StatusEffectInstance;

pub fn tint_status_effects(
    instances: Query<&StatusEffectInstance>,
    enemies: Query<(Entity, &EnemyAppearance, &MeshMaterial2d<ColorMaterial>), With<Enemy>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Dominant tinted status per target (last one iterated wins — a coarse but stable cue).
    let mut tint: HashMap<Entity, Color> = HashMap::new();
    for inst in &instances {
        if let Some(color) = status_tint(&inst.def_id) {
            tint.insert(inst.target, color);
        }
    }

    for (entity, appearance, material) in &enemies {
        let color = tint.get(&entity).copied().unwrap_or(appearance.color);
        if let Some(mat) = materials.get_mut(&material.0) {
            mat.color = color;
        }
    }
}

/// Maps a status id to its overlay tint. `None` = the status has no visual tint (it doesn't
/// override the archetype color).
fn status_tint(def_id: &str) -> Option<Color> {
    match def_id {
        "frostbite" => Some(Color::srgb(0.5, 0.75, 1.0)),
        "blaze" => Some(Color::srgb(1.0, 0.5, 0.15)),
        "bleed" => Some(Color::srgb(0.8, 0.1, 0.1)),
        "root" | "stun" => Some(Color::srgb(0.75, 0.75, 0.25)),
        _ => None,
    }
}
