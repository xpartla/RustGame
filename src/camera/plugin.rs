use bevy::prelude::*;
use crate::camera::systems::{draw_cursor, setup_camera, setup_scene, update_camera};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self,app: &mut App) {
        app
            .add_systems(Startup, (setup_scene, setup_camera))
            .add_systems(Update, update_camera)
            .add_systems(PostUpdate, draw_cursor.after(TransformSystem::TransformPropagate));
    }
}