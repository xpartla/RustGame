// TODO(Phase 4): Wire into GamePlugin.
//
// Responsibilities:
//   - Registers HeroDef as a Bevy asset + loader
//   - Adds resolve_input_to_ability before CombatSet::Damage
//   - Adds handle_stance_swap before resolve_input_to_ability
//   - All systems run in InState(GameState::InRun)

use bevy::prelude::*;

pub struct HeroPlugin;

impl Plugin for HeroPlugin {
    fn build(&self, _app: &mut App) {
        todo!("Phase 4")
    }
}
