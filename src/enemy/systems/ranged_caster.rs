use bevy::prelude::*;
use crate::core::components::{Facing, FlowField, GridPosition, Velocity, WorldPosition};
use crate::enemy::components::{AiBehavior, Enemy, MoveSpeed};
use crate::player::components::Player;

/// Drives `RangedCaster` enemies (Phase 5): approach the player along the flow field until within
/// `preferred_range`, then stop and hold. Always faces the player — independent of velocity — so
/// the aim-dependent `projectile` ability can fire while standing still (`update_enemy_facing`
/// skips non-chasers, so it does not overwrite this aim). Inert until a ranged enemy exists.
pub fn ranged_caster_ai(
    flow_field: Res<FlowField>,
    player: Query<&WorldPosition, With<Player>>,
    mut enemies: Query<
        (&GridPosition, &WorldPosition, &mut Velocity, &mut Facing, &MoveSpeed, &AiBehavior),
        With<Enemy>,
    >,
) {
    let Ok(player_pos) = player.single() else {
        return;
    };
    for (grid, pos, mut vel, mut facing, speed, ai) in &mut enemies {
        let AiBehavior::RangedCaster { preferred_range } = ai else {
            continue;
        };
        let to_player = player_pos.0 - pos.0;
        if to_player.length_squared() > 1e-6 {
            facing.0 = to_player.normalize();
        }
        if to_player.length() <= *preferred_range {
            // In range: stand and shoot.
            vel.0 = Vec2::ZERO;
        } else if let Some(direction) = flow_field.direction.get(grid) {
            let desired = direction.normalize_or_zero() * speed.0;
            vel.0 = vel.0.lerp(desired, 0.2);
        } else {
            vel.0 = Vec2::ZERO;
        }
    }
}
