use bevy::app::PostUpdate;
use bevy::prelude::{App, Plugin, Startup};
use crate::enemy::systems::{
    spawner::spawn_enemy,
    debug::draw_enemy_world_positions
};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_enemy)
            .add_systems(PostUpdate, draw_enemy_world_positions);
    }
}
