// Translates player input + active stance → TriggerAbilityEvent.
//
// This is the indirection layer between raw key/mouse input and ability execution.
// It knows nothing about what the ability does — it only resolves which ability is
// currently mapped to the pressed slot and emits TriggerAbilityEvent for it.
//
// Resolution path:
//   1. Read the pressed InputSlot from mouse/keyboard input (LMB → Basic, RMB → Special, etc.)
//   2. Read player's ActiveStance.
//   3. Look up HeroDef.stance_slots for the matching (stance, slot) pair → AbilityId.
//   4. Emit TriggerAbilityEvent { ability_id, owner }.
//
// StanceSwap (Q) is handled separately by hero/systems/stance.rs, not here.
//
// Runs before CombatSet::Damage so the event is available to execute_ready_abilities.

use bevy::prelude::*;
use crate::ability::components::TriggerAbilityEvent;
use crate::hero::assets::HeroDef;
use crate::hero::components::{ActiveStance, HeroIdentity, InputSlot};

/// TODO(Phase 4): implement.
/// Query: player entity with (HeroIdentity, ActiveStance);
///        HeroDef assets; keyboard/mouse input.
pub fn resolve_input_to_ability(
    _kb: Res<ButtonInput<KeyCode>>,
    _mouse: Res<ButtonInput<MouseButton>>,
    _player: Query<(Entity, &HeroIdentity, &ActiveStance)>,
    _hero_defs: Res<Assets<HeroDef>>,
    mut _trigger_events: EventWriter<TriggerAbilityEvent>,
) {
    // Phase 1 note: until HeroDef assets are loaded, use a temporary hardcoded mapping
    // that routes LMB → "death_strike", RMB → "dnd", etc. This lets the ability system
    // be tested without the full hero asset pipeline.
    todo!("Phase 4 (Phase 1 uses temporary hardcoded stub)")
}
