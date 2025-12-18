use bevy::prelude::*;
use crate::player::systems::input::player_input;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
        fn build(&self, app: &mut App) {
        app
            .add_systems(Update, player_input);
    }
}