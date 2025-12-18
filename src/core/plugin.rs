use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update};
use crate::core::systems::{
    movement::apply_velocity,
    render_sync::sync_transform,
    grid_sync::world_to_grid
};

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app:&mut App) {
        app.add_systems(Update,
                        (
                            apply_velocity,
                            world_to_grid.after(apply_velocity),
                            sync_transform.after(world_to_grid),
                            ),
        );
    }
}