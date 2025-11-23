use bevy::core_pipeline::bloom::Bloom;
use bevy::prelude::{Assets, Camera2d, Circle, Color, Commands, Mesh, Mesh2d, MeshMaterial2d, Rectangle, Res, ResMut, Single, StableInterpolate, Time, Transform, Vec3, With, Without};
use bevy::sprite::ColorMaterial;
use crate::player::components::Player;
use crate::camera::constants::CAMERA_DECAY;

pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>
) {
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1000., 700.))),
        MeshMaterial2d(materials.add(Color::srgb(0.2, 0.2, 0.3))),
        ));
    commands.spawn((
        Player,
        Mesh2d(meshes.add(Circle::new(25.))),
        MeshMaterial2d(materials.add(Color::srgb(6.25, 9.4, 9.1))),
        Transform::from_xyz(0., 0., 2.),
        ));
}

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Bloom::NATURAL));
}

pub fn update_camera(
    mut camera: Single<&mut Transform, (With<Camera2d>, Without<Player>)>,
    player: Single<&Transform, (With<Player>, Without<Camera2d>)>,
    time: Res<Time>,
) {
    let Vec3 {x, y, ..} = player.translation;
    let direction = Vec3::new(x,y, camera.translation.z);
    camera
        .translation
        .smooth_nudge(&direction, CAMERA_DECAY, time.delta_secs());
}