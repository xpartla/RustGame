use bevy::prelude::{Query, Res, Vec2, With};
use crate::core::components::{FlowField, GridPosition, MoveSpeed, Velocity, WorldPosition};
use crate::enemy::components::{AiBehavior, Enemy, Taunted};

/// `Taunted` (Phase 9.4 — Ent taunt) overrides the flow field entirely: a taunted chaser steers
/// straight-line toward its taunt source instead, mirroring
/// `ability::systems::summon::minion_seek_and_face`'s own straight-line chase (same reason — the
/// shared `FlowField` only ever points toward the PLAYER, exactly wrong for "go fight the Ent").
pub fn enemy_follow_flow_field(
    flow_field: Res<FlowField>,
    taunters: Query<&WorldPosition>,
    mut enemies: Query<
        (&GridPosition, &WorldPosition, &mut Velocity, &MoveSpeed, &AiBehavior, Option<&Taunted>),
        With<Enemy>,
    >,
) {
    for (grid_pos, pos, mut vel, speed, ai, taunted) in &mut enemies {
        // Only flow-field chasers steer here; ranged/stationary AI drive their own movement.
        if !matches!(ai, AiBehavior::MeleeChaser) {
            continue;
        }
        if let Some(taunt_pos) = taunted.and_then(|t| taunters.get(t.0).ok()) {
            let to_taunter = taunt_pos.0 - pos.0;
            vel.0 = if to_taunter.length_squared() > 1e-6 {
                to_taunter.normalize() * speed.0
            } else {
                Vec2::ZERO
            };
            continue;
        }
        if let Some(direction) = flow_field.direction.get(grid_pos) {
            let desired = direction.normalize_or_zero() * speed.0;
            vel.0 = vel.0.lerp(desired, 0.2);
        } else {
            vel.0 = Vec2::ZERO;
        }
    }
}
