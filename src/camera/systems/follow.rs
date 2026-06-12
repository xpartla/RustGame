use bevy::prelude::{Camera2d, Query, Res, StableInterpolate, Time, Transform, With};
use crate::camera::constants::CAMERA_DECAY;
use crate::core::components::WorldPosition;
use crate::player::components::Player;

pub fn follow_player(
    player: Query<&WorldPosition, With<Player>>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
    time: Res<Time>,
) {
    let Ok(player_pos) = player.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };

    let target = player_pos.0.extend(camera_transform.translation.z);

    camera_transform
        .translation
        .smooth_nudge(&target, CAMERA_DECAY, time.delta_secs());
}