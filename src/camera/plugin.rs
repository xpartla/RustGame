use bevy::prelude::*;
use crate::camera::systems::systems::{draw_cursor, setup_camera, setup_scene};
use crate::camera::systems::follow::follow_player;
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self,app: &mut App) {
        app
            .add_systems(Startup, (setup_scene, setup_camera))
            .add_systems(Update, follow_player)
            .add_systems(PostUpdate, draw_cursor.after(TransformSystem::TransformPropagate));
    }
}