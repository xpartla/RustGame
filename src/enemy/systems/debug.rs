use bevy::prelude::{Entity, Gizmos, Query, With};
use crate::ability::components::{AbilityCooldown, AbilityInstance};
use crate::core::components::WorldPosition;
use crate::enemy::components::Enemy;
use crate::player::components::Player;
use bevy::color::palettes::css::RED;
use crate::constants::ENEMY_ATTACK_FLASH_SECS;

/// Flashes a red strike line (enemy → player) plus a ring on the enemy right after it lands a hit.
/// Reads the enemy's contact/attack ability cooldown (Phase 5: contact melee is an auto-cast
/// ability): a cooldown that has fired (`duration > 0`) with a small `elapsed` means "just attacked".
pub fn draw_enemy_attack_flash(
    enemies: Query<(Entity, &WorldPosition), With<Enemy>>,
    instances: Query<(&AbilityInstance, &AbilityCooldown)>,
    player: Query<&WorldPosition, With<Player>>,
    mut gizmos: Gizmos,
) {
    let Ok(player_pos) = player.single() else {
        return;
    };

    for (enemy, pos) in &enemies {
        let just_attacked = instances.iter().any(|(inst, cd)| {
            inst.owner == enemy && cd.duration > 0.0 && cd.elapsed < ENEMY_ATTACK_FLASH_SECS
        });
        if just_attacked {
            gizmos.line_2d(pos.0, player_pos.0, RED);
            gizmos.circle_2d(pos.0, 12.0, RED);
        }
    }
}
