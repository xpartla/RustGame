// Library crate root. All game code lives in the library so that:
//   - the windowed binary (src/main.rs) stays a thin launcher,
//   - integration tests (tests/) can drive the game headlessly through src/sim/,
//   - future tooling (balance arena binary) can reuse the same plugins.
//
// Scaffold modules not yet wired into the build (hero, status, zone, meta, and most of run/)
// are intentionally NOT declared here — same as before the lib split. They join the crate in
// their own phases.

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
pub mod progression;
pub mod ui;
pub mod sim;
