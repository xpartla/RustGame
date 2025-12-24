use bevy::app::PostUpdate;
use bevy::prelude::{App, Plugin, Startup, Timer, TimerMode, Update};
use crate::enemy::components::EnemySpawner;
use crate::enemy::systems::{
    debug::draw_enemy_world_positions
};
use crate::enemy::systems::spawner::spawn_enemy_over_time;
use crate::enemy::systems::death::enemy_death;
pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(EnemySpawner {
                timer: Timer::from_seconds(5.0, TimerMode::Repeating),
                radius: 10,
            })
            .add_systems(Update, spawn_enemy_over_time)
            .add_systems(Update, enemy_death)
            .add_systems(PostUpdate, draw_enemy_world_positions);
    }
}
