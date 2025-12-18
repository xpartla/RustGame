use bevy::app::PostUpdate;
use bevy::prelude::{App, Plugin, Startup, Update};
use crate::enemy::systems::{
    chase_player::chase_player,
    spawner::spawn_enemy,
    debug::draw_enemy_grid_positions,
    debug::draw_enemy_world_positions
};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_enemy)
            .add_systems(Update, chase_player)
            .add_systems(PostUpdate, draw_enemy_world_positions);
    }
}
