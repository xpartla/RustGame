// TODO(Phase 6): Wire into GamePlugin.
//
// Responsibilities:
//   - Inserts PlayerZonePresence resource
//   - Adds build_player_zone_presence at start of Update (before CombatSet)
//   - Adds tick_zone_lifetimes and move_anchored_zones in Update
//   - All systems run in InState(GameState::InRun)

use bevy::prelude::*;

pub struct ZonePlugin;

impl Plugin for ZonePlugin {
    fn build(&self, _app: &mut App) {
        todo!("Phase 6")
    }
}
