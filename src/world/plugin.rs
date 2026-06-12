use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Startup};
use crate::world::components::TileMap;
use crate::world::systems::generate_map::generate_map;
use crate::world::systems::render_map::render_map;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<TileMap>()
            .add_systems(Startup, (generate_map, render_map.after(generate_map)));
    }
}
