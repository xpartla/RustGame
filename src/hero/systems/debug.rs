// Debug-only playtest aid (Phase 4): press M to re-identify the live player as the Mage.
//
// Mirrors progression::systems::level_up::debug_force_level_up (press L). Lets the class be felt
// on the Windows build without a character-select screen (deferred). Compiled out of release
// builds via #[cfg(debug_assertions)]; the golden-master campaign bot never presses M, so the
// baseline is untouched.
//
// It sets HeroIdentity → "mage" + ActiveStance → "fire" and removes the Level1Granted marker, so
// the deferred grant (ability/plugin.rs::grant_level_1_abilities) re-runs next frame and hands
// the player the Mage's level-1 abilities (fireblast + frostbolt). The old Death Knight ability
// instances remain but are simply unbound by the Mage's stance slots.

use bevy::prelude::*;
use crate::ability::components::Level1Granted;
use crate::hero::components::{ActiveStance, HeroIdentity};
use crate::player::components::Player;

pub fn debug_swap_to_mage(
    kb: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut player: Query<(Entity, &mut HeroIdentity, &mut ActiveStance), With<Player>>,
) {
    if !kb.just_pressed(KeyCode::KeyM) {
        return;
    }
    for (entity, mut hero_id, mut stance) in &mut player {
        hero_id.0 = "mage".to_string();
        stance.0 = "fire".to_string();
        commands.entity(entity).remove::<Level1Granted>();
        info!("[debug] swapped player to Mage (fire stance)");
    }
}
