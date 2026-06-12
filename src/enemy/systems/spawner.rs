use bevy::math::Vec2;
use bevy::prelude::{Commands, Res, ResMut, Transform};
use bevy::time::Time;
use rand::Rng;
use crate::constants::TILE_SIZE;
use crate::constants::ENEMY_HEALTH;
use crate::core::components::{GridPosition, Health, Velocity, WorldPosition};
use crate::enemy::components::{Enemy, EnemySpawner};

pub fn spawn_enemy_over_time(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<EnemySpawner>,
){
    spawner.timer.tick(time.delta());
    if(!spawner.timer.finished()) {
        return;
    }

    let mut rng = rand::thread_rng();
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let dist = spawner.radius as f32;

    let x = (angle.cos() * dist) as i32;
    let y = (angle.sin() * dist) as i32;

    commands.spawn((
        Enemy,
        Health::new(ENEMY_HEALTH),
        GridPosition { x, y },
        WorldPosition(Vec2::new(x as f32*TILE_SIZE, y as f32*TILE_SIZE)),
        Velocity::default(),
        Transform::default(),
        ));

}
