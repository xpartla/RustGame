use bevy::asset::Assets;
use bevy::color::Color;
use bevy::math::Vec2;
use bevy::prelude::{Circle, ColorMaterial, Commands, Mesh, Mesh2d, MeshMaterial2d, ResMut, Transform};
use crate::constants::PLAYER_HEALTH;
use crate::core::components::{GridPosition, Health, Velocity, WorldPosition};
use crate::player::components::{Facing, Player};

pub fn spawn_player(mut commands: Commands,
                    mut meshes: ResMut<Assets<Mesh>>,
                    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Player,
        Health::new(PLAYER_HEALTH),
        WorldPosition(Vec2::ZERO),
        GridPosition{x:0, y:0},
        Facing(Vec2::default()),
        Velocity::default(),
        Transform::from_xyz(0., 0., 2.),
        Mesh2d(meshes.add(Circle::new(25.))),
        MeshMaterial2d(materials.add(Color::srgb(6.25, 9.4, 9.1))),
    ));
}