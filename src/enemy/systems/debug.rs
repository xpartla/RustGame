use bevy::prelude::{Gizmos, Query, With};
use crate::core::components::WorldPosition;
use crate::enemy::components::{AttackCooldown, Enemy};
use crate::player::components::Player;
use bevy::color::palettes::css::RED;
use crate::constants::ENEMY_ATTACK_FLASH_SECS;

/// Flashes a red strike line (enemy → player) plus a ring on the enemy right after it lands a
/// hit. Stateless: an enemy's `AttackCooldown` only resets to 0 on a successful attack, so a
/// small elapsed time means "just attacked".
pub fn draw_enemy_attack_flash(
    enemies: Query<(&WorldPosition, &AttackCooldown), With<Enemy>>,
    player: Query<&WorldPosition, With<Player>>,
    mut gizmos: Gizmos,
) {
    let Ok(player_pos) = player.single() else {
        return;
    };

    for (pos, cooldown) in &enemies {
        if cooldown.timer.elapsed_secs() < ENEMY_ATTACK_FLASH_SECS {
            gizmos.line_2d(pos.0, player_pos.0, RED);
            gizmos.circle_2d(pos.0, 12.0, RED);
        }
    }
}
