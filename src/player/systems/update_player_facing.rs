use bevy::prelude::{Camera, GlobalTransform, Query, With};
use bevy::window::Window;
use crate::core::components::WorldPosition;
use crate::player::components::{Facing, Player};

pub fn update_player_facing(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut player_q: Query<(&WorldPosition, &mut Facing), With<Player>>,
) {
    let window = match windows.single() {
        Ok(w) => w,
        Err(_) => return,
    };

    let cursor_pos = match window.cursor_position() {
        Some(p) => p,
        None => return,
    };

    let (camera, camera_transform) = match camera_q.single() {
        Ok(v) => v,
        Err(_) => return,
    };

    let world_cursor = match camera.viewport_to_world_2d(
        camera_transform,
        cursor_pos,
    ) {
        Ok(p) => p,
        Err(_) => return,
    };

    let (player_pos, mut facing) = match player_q.single_mut() {
        Ok(v) => v,
        Err(_) => return,
    };

    let dir = world_cursor - player_pos.0;
    if dir.length_squared() > 0.0001 {
        facing.0 = dir.normalize();
    }
}
