// Windowed launcher. All game code lives in the library crate (src/lib.rs); headless
// simulation and tests use rust_game::sim instead of DefaultPlugins.

use bevy::DefaultPlugins;
use bevy::prelude::App;
use rust_game::game::GamePlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(GamePlugin);
    app.run();
}
