use bevy::prelude::{Commands, Entity, Query, Res, Vec2};
use bevy::time::Time;
use crate::core::components::{ForcedImpulse, GridPosition, Immobilized, MoveSpeedModifier, Velocity, WorldPosition, ZoneSpeedModifier};
use crate::world::components::TileMap;

/// Resolves an active `ForcedImpulse` (Phase 9.1, §8.1(6)): overwrites the entity's `Velocity` with
/// the impulse's velocity — overriding whatever `MovementSet::Intent` (flow-field AI, WASD input)
/// set this frame — and ticks its timer down, removing the impulse once it finishes. Runs first in
/// `MovementSet::Integrate`, ahead of `apply_velocity`, so the overridden `Velocity` still goes
/// through the normal per-axis `TileMap` wall-slide.
pub fn resolve_forced_movement(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Velocity, &mut ForcedImpulse)>,
) {
    let dt = time.delta();
    for (entity, mut vel, mut impulse) in &mut query {
        vel.0 = impulse.velocity;
        impulse.timer.tick(dt);
        if impulse.timer.finished() {
            commands.entity(entity).remove::<ForcedImpulse>();
        }
    }
}

/// Advances every `WorldPosition` by its `Velocity`, blocked by impassable tiles.
///
/// Movement is resolved **per axis** against the `TileMap`: the X step is applied only if the
/// destination tile is walkable, then the Y step independently. This lets entities slide along a
/// wall instead of sticking when they push into it diagonally. Collision is tested at the
/// entity's *center* tile (radius-aware collision is a future refinement). Applies uniformly to
/// the player and enemies, since both share this component pair.
///
/// Status modifiers ride on generic components (Phase 3): the integration step is scaled by
/// `MoveSpeedModifier` (frostbite slow) and skipped entirely while `Immobilized` (root/stun).
/// `ZoneSpeedModifier` (Phase 9.2) is a second, independent multiplier folded in the same way —
/// see its own doc comment for why it isn't just folded into `MoveSpeedModifier`.
/// Scaling the *step* — not the stored `Velocity` — keeps the enemy-AI lerp toward its desired
/// velocity clean. Entities without these components are unaffected (step = `vel * dt`).
pub fn apply_velocity(
    mut query: Query<(&mut WorldPosition, &Velocity, Option<&MoveSpeedModifier>, Option<&ZoneSpeedModifier>, Option<&Immobilized>)>,
    map: Res<TileMap>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    for (mut pos, vel, move_mod, zone_mod, immobilized) in &mut query {
        if immobilized.is_some() {
            continue;
        }
        let speed_mult = move_mod.map(|m| m.0).unwrap_or(1.0) * zone_mod.map(|m| m.0).unwrap_or(1.0);
        let step = vel.0 * delta * speed_mult;

        let try_x = Vec2::new(pos.0.x + step.x, pos.0.y);
        if !map.is_blocked(GridPosition::from_world(try_x)) {
            pos.0.x = try_x.x;
        }

        let try_y = Vec2::new(pos.0.x, pos.0.y + step.y);
        if !map.is_blocked(GridPosition::from_world(try_y)) {
            pos.0.y = try_y.y;
        }
    }
}
