// PHASE 1 STUB: hardcoded input → ability routing.
//
// Left mouse button fires the Basic-slot ability (Death Strike). This is the temporary
// stand-in for the hero indirection layer: Phase 4's hero/systems/input_slot.rs will read
// the player's ActiveStance and HeroDef.stance_slots to resolve the pressed InputSlot to an
// AbilityId, and will also bind Special (RMB) / Movement. Until then, this keeps the ability
// pipeline exercisable without the hero asset pipeline.
//
// Runs before CombatSet::Damage so the TriggerAbilityEvent is available to the execution
// system the same frame.

use bevy::prelude::*;
use crate::ability::components::TriggerAbilityEvent;
use crate::player::components::Player;

pub fn player_ability_input(
    mouse: Res<ButtonInput<MouseButton>>,
    players: Query<Entity, With<Player>>,
    mut triggers: EventWriter<TriggerAbilityEvent>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    for owner in &players {
        triggers.write(TriggerAbilityEvent {
            ability_id: "death_strike".to_string(),
            owner,
        });
    }
}
