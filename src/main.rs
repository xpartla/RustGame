mod constants;
mod camera;
mod player;
mod game;
mod core;
mod enemy;
mod projectile;
mod pickup;
mod world;

use bevy::DefaultPlugins;
use bevy::prelude::{App};
// use crate::camera::CameraPlugin;
use crate::game::GamePlugin;
// use crate::player::PlayerPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    // app.add_plugins(CameraPlugin);
    // app.add_plugins(PlayerPlugin);
    app.add_plugins(GamePlugin);
    app.run();
    // println!("Hello, world!");
}
