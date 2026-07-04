use bevy::prelude::{App, Plugin, Startup};
use crate::world::components::TileMap;
use crate::world::systems::generate_map::generate_map;

// Presentation note: render_map (floor + obstacle meshes) is registered by
// game::presentation::PresentationPlugin, ordered after generate_map as before.

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<TileMap>()
            .add_systems(Startup, generate_map);
    }
}
