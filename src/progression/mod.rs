// Phase 2: Leveling and talent-offer flow.
//
// NOT yet wired into main.rs or GamePlugin. Add in Phase 2.
//
// This module consumes LevelUpEvent (emitted by core/systems/experience.rs) and drives
// the two-phase level-up flow:
//   Phase AbilityUnlock (L2–L6): pop one ability from the band pool, emit UnlockAbilityEvent.
//   Phase TalentChoices (all abilities unlocked): generate a 3-option talent offer,
//     push GameState::TalentPicker.
//
// ThroneRoom rewards also funnel through generate_offer() with RarityFilter::RareOrAbove,
// ensuring a consistent code path for all talent acquisition.
//
// Module map:
//   state.rs  — LevelUpFlowState resource, LevelUpPhase, TalentOffer, RarityFilter
//   systems/
//     level_up.rs — consumes LevelUpEvent, drives phase transitions, emits UnlockAbilityEvent
//     offer.rs    — generate_offer() — samples talent pool respecting uniqueness constraints

pub mod plugin;
pub mod state;
pub mod systems;
