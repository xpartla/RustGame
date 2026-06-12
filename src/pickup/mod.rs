pub(crate) mod components;
pub(crate) mod constants;
pub(crate) mod systems;
mod plugin;

pub use plugin::PickUpPlugin;
/// Re-exported so other features (e.g. `enemy_death` drops) can spawn pickups without reaching
/// into the systems submodule.
pub use systems::spawn_pickups::spawn_pickup;
