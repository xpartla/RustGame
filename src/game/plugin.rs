use bevy::app::{App, Plugin};
use bevy::prelude::*;
use crate::player::systems::move_player;
use crate::camera::systems::update_camera;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (move_player, update_camera).chain());
    }
}