use bevy::color::palettes::css::{RED, WHITE};
use bevy::core_pipeline::bloom::Bloom;
use bevy::prelude::{Camera, Camera2d, Commands, Gizmos, GlobalTransform, Single, Vec2Swizzles, With};
use bevy::window::Window;


pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Bloom::NATURAL));
}

pub fn draw_cursor(camera_query: Single<(&Camera, &GlobalTransform), With<Camera2d>>, window: Single<&Window>, mut gizmos: Gizmos) {
    let (camera, camera_transform) = *camera_query;
    if let Some(cursor_position) = window.cursor_position()
        && let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position)
        && let Ok(viewport_check) = camera.world_to_viewport(camera_transform, world_pos.extend(0.0))
        && let Ok(world_check) = camera.viewport_to_world_2d(camera_transform, viewport_check.xy())
    {
        gizmos.circle_2d(world_pos, 10., WHITE);
        gizmos.circle_2d(world_check, 8., RED);
    }
}
