use bevy::app::{App, Plugin};
use crate::camera::CameraPlugin;
use crate::core::CorePlugin;
use crate::enemy::EnemyPlugin;
use crate::pickup::PickUpPlugin;
use crate::player::PlayerPlugin;
use crate::projectile::ProjectilePlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins((
                CorePlugin,
                PlayerPlugin,
                EnemyPlugin,
                ProjectilePlugin,
                PickUpPlugin,
                CameraPlugin,
            ));
    }
}