// LevelUpFlowState — tracks where the player is in the level-up progression for this run.
//
// Stored inline in RunState so it is persisted and resumed correctly.
//
// Phase transitions:
//   AbilityUnlock phase:
//     - L2/L3: draw one ability from band_2_3_remaining (shuffled at run start with RunRng).
//     - L4/L5/L6: draw one from band_4_6_remaining.
//     - When both pools are empty: transition to TalentChoices phase.
//   TalentChoices phase:
//     - Each level-up: call generate_offer() → store pending_offer → push GameState::TalentPicker.
//     - Player picks (or declines) → talent/systems/apply.rs handles acquisition.
//
// ThroneRoom rewards use generate_offer() with OfferContext::ThroneRoom (RarityFilter::RareOrAbove).
// This goes through the same pending_offer / TalentPicker flow.

use crate::ability::assets::AbilityId;
use crate::talent::offer::TalentOffer;

/// Stored in RunState; serialized with it.
#[derive(Debug, Clone)]
pub struct LevelUpFlowState {
    pub phase: LevelUpPhase,
    /// Shuffled at run start. Pop from the front to draw without replacement.
    pub band_2_3_remaining: Vec<AbilityId>,
    pub band_4_6_remaining: Vec<AbilityId>,
    /// Set when a level-up generates an offer; cleared after the player responds.
    /// Also set for ThroneRoom rewards.
    pub pending_offer: Option<TalentOffer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelUpPhase {
    /// Levels 2–6: unlock core abilities from band pools.
    AbilityUnlock,
    /// All core abilities unlocked: subsequent levels offer talent choices.
    TalentChoices,
}

impl LevelUpFlowState {
    /// Initialize from the hero's band pools at run start. Shuffles both pools using RunRng.
    /// TODO(Phase 2): accept &mut RunRng and shuffle in-place.
    pub fn new(band_2_3: Vec<AbilityId>, band_4_6: Vec<AbilityId>) -> Self {
        Self {
            phase: LevelUpPhase::AbilityUnlock,
            band_2_3_remaining: band_2_3,
            band_4_6_remaining: band_4_6,
            pending_offer: None,
        }
    }
}
