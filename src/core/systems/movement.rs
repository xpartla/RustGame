use bevy::prelude::{Query, Res, Vec2};
use bevy::time::Time;
use crate::core::components::{GridPosition, Velocity, WorldPosition};
use crate::world::components::TileMap;

/// Advances every `WorldPosition` by its `Velocity`, blocked by impassable tiles.
///
/// Movement is resolved **per axis** against the `TileMap`: the X step is applied only if the
/// destination tile is walkable, then the Y step independently. This lets entities slide along a
/// wall instead of sticking when they push into it diagonally. Collision is tested at the
/// entity's *center* tile (radius-aware collision is a future refinement). Applies uniformly to
/// the player and enemies, since both share this component pair.
pub fn apply_velocity(
    mut query: Query<(&mut WorldPosition, &Velocity)>,
    map: Res<TileMap>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    for (mut pos, vel) in &mut query {
        let step = vel.0 * delta;

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
