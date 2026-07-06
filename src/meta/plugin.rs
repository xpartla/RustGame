// MetaPlugin — inserts the MetaState resource so meta-progression logic is sim-able (Phase 8, §2).
//
// Joins GameLogicPlugin unconditionally, including the headless sim: `MetaState::default()` (every
// hero unlocked, no history, no save) requires no disk access, so this is safe in any sim/test.
//
// The windowed game's *disk* I/O — loading the real save at boot (overriding the in-memory default)
// and autosaving on every change — is wired separately by GamePlugin (game/plugin.rs), using the
// thin wrappers in meta/persistence.rs. The sim never touches a filesystem.

use bevy::prelude::*;
use crate::meta::state::MetaState;

pub struct MetaPlugin;

impl Plugin for MetaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MetaState>();
    }
}
