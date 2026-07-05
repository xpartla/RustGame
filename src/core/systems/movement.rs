use bevy::prelude::{Query, Res, Vec2};
use bevy::time::Time;
use crate::core::components::{GridPosition, Immobilized, MoveSpeedModifier, Velocity, WorldPosition};
use crate::world::components::TileMap;

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
/// Scaling the *step* — not the stored `Velocity` — keeps the enemy-AI lerp toward its desired
/// velocity clean. Entities without these components are unaffected (step = `vel * dt`).
pub fn apply_velocity(
    mut query: Query<(&mut WorldPosition, &Velocity, Option<&MoveSpeedModifier>, Option<&Immobilized>)>,
    map: Res<TileMap>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    for (mut pos, vel, move_mod, immobilized) in &mut query {
        if immobilized.is_some() {
            continue;
        }
        let speed_mult = move_mod.map(|m| m.0).unwrap_or(1.0);
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
