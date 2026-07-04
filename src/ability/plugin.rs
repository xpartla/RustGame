// TODO(Phase 1): Wire into GamePlugin.
//
// Responsibilities:
//   - Registers BehaviorRegistry resource with built-in behaviors
//   - Registers HookRegistry resource
//   - Registers TriggerAbilityEvent
//   - Adds execute_abilities and spawn_ability_instances systems
//   - All ability systems run in InState(GameState::InRun)

use bevy::prelude::*;

pub struct AbilityPlugin;

impl Plugin for AbilityPlugin {
    fn build(&self, _app: &mut App) {
        // TODO(Phase 1):
        //   app.init_resource::<BehaviorRegistry>();
        //   app.init_resource::<HookRegistry>();
        //   app.add_event::<TriggerAbilityEvent>();
        //   app.add_systems(Update, (
        //       crate::ability::systems::execute::tick_ability_cooldowns,
        //       crate::ability::systems::execute::execute_ready_abilities,
        //   ).in_set(CombatSet::Damage).run_if(in_state(GameState::InRun)));
        todo!("Phase 1")
    }
}
