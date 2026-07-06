// Library crate root. All game code lives in the library so that:
//   - the windowed binary (src/main.rs) stays a thin launcher,
//   - integration tests (tests/) can drive the game headlessly through src/sim/,
//   - future tooling (balance arena binary) can reuse the same plugins.
//
// Every domain module is now live (meta joined in Phase 8; hero in Phase 4; zone in Phase 6).

pub mod constants;
pub mod camera;
pub mod player;
pub mod game;
pub mod core;
pub mod enemy;
pub mod projectile;
pub mod pickup;
pub mod world;
pub mod run;
pub mod ability;
pub mod status;
pub mod talent;
pub mod hero;
pub mod progression;
pub mod zone;
pub mod meta;
pub mod ui;
pub mod sim;
