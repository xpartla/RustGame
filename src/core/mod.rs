// Visibility note: modules are `pub` (not `pub(crate)`) so integration tests (tests/) can
// reach components/events through the library crate. Same applies across all domain modules.
pub mod components;
pub mod def_library;
pub mod events;
pub mod sets;
pub mod systems;
mod constants;
mod plugin;

pub use plugin::CorePlugin;
