use bevy::asset::Assets;
use bevy::prelude::{Commands, Mesh, Mesh2d, MeshMaterial2d, Rectangle, Res, ResMut, Transform};
use bevy::sprite::ColorMaterial;
use crate::constants::TILE_SIZE;
use crate::world::components::TileMap;
use crate::world::constants::{FLOOR_COLOR, OBSTACLE_COLOR, OBSTACLE_Z};

/// Renders the map (Startup, after `generate_map`): a single floor rectangle spanning the whole
/// map (this replaces the old static camera backdrop) plus one square mesh per blocked tile.
///
/// All obstacle tiles share a single mesh + material handle — only their `Transform` differs.
/// These are static entities (no `WorldPosition`), so `sync_transform` leaves them alone.
pub fn render_map(
    mut commands: Commands,
    map: Res<TileMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let floor_w = (map.half_width * 2 + 1) as f32 * TILE_SIZE;
    let floor_h = (map.half_height * 2 + 1) as f32 * TILE_SIZE;
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(floor_w, floor_h))),
        MeshMaterial2d(materials.add(FLOOR_COLOR)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let tile_mesh = meshes.add(Rectangle::new(TILE_SIZE, TILE_SIZE));
    let tile_material = materials.add(OBSTACLE_COLOR);
    for tile in &map.blocked {
        commands.spawn((
            Mesh2d(tile_mesh.clone()),
            MeshMaterial2d(tile_material.clone()),
            Transform::from_xyz(
                tile.x as f32 * TILE_SIZE,
                tile.y as f32 * TILE_SIZE,
                OBSTACLE_Z,
            ),
        ));
    }
}
