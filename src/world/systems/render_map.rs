use bevy::asset::Assets;
use bevy::prelude::{Commands, Component, Entity, Mesh, Mesh2d, MeshMaterial2d, Query, Rectangle, Res, ResMut, Transform, With};
use bevy::sprite::ColorMaterial;
use crate::constants::TILE_SIZE;
use crate::world::components::TileMap;
use crate::world::constants::{FLOOR_COLOR, OBSTACLE_COLOR, OBSTACLE_Z};

/// Marks the floor + obstacle meshes so `rerender_map` can clear them when the map regenerates
/// between encounters (Phase 7). Static entities (no `WorldPosition`), so `sync_transform` ignores them.
#[derive(Component)]
pub struct MapRendered;

/// Renders the map (Startup, after `generate_map`): a single floor rectangle spanning the whole
/// map (this replaces the old static camera backdrop) plus one square mesh per blocked tile.
///
/// All obstacle tiles share a single mesh + material handle — only their `Transform` differs.
pub fn render_map(
    mut commands: Commands,
    map: Res<TileMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    spawn_map_meshes(&mut commands, &map, &mut meshes, &mut materials);
}

/// Re-renders the map whenever the `TileMap` changes (Phase 7 — `load_encounter` regenerates it per
/// encounter): despawn the previous floor/obstacle meshes and rebuild from the new tiles. Presentation
/// only (never runs headless), so it does not affect the golden master. Gated on `resource_changed`
/// by `PresentationPlugin`; the Startup insert is covered by `render_map`, so this only fires on real
/// regenerations.
pub fn rerender_map(
    mut commands: Commands,
    map: Res<TileMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    existing: Query<Entity, With<MapRendered>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    spawn_map_meshes(&mut commands, &map, &mut meshes, &mut materials);
}

fn spawn_map_meshes(
    commands: &mut Commands,
    map: &TileMap,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    let floor_w = (map.half_width * 2 + 1) as f32 * TILE_SIZE;
    let floor_h = (map.half_height * 2 + 1) as f32 * TILE_SIZE;
    commands.spawn((
        MapRendered,
        Mesh2d(meshes.add(Rectangle::new(floor_w, floor_h))),
        MeshMaterial2d(materials.add(FLOOR_COLOR)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let tile_mesh = meshes.add(Rectangle::new(TILE_SIZE, TILE_SIZE));
    let tile_material = materials.add(OBSTACLE_COLOR);
    for tile in &map.blocked {
        commands.spawn((
            MapRendered,
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
