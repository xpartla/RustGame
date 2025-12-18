use bevy::app::{App, Plugin};
use crate::camera::CameraPlugin;
use crate::core::CorePlugin;
use crate::enemy::EnemyPlugin;
use crate::player::PlayerPlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins((
                CorePlugin,
                PlayerPlugin,
                EnemyPlugin,
                CameraPlugin,
            ));
    }
}