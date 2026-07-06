// LevelUpFlowState — tracks where the player is in the level-up progression for this run.
//
// Phase 2: inserted as a standalone Resource by ProgressionPlugin (seeded from hardcoded BDK
// band pools, shuffled with RunRng). Phase 7 will store it inline in RunState so it is
// persisted and resumed; the struct still derives Resource then (Resource is just a marker,
// harmless as a nested field).
//
// Phase transitions:
//   AbilityUnlock phase:
//     - L2/L3: draw one ability from band_2_3_remaining (shuffled at run start with RunRng).
//     - L4/L5/L6: draw one from band_4_6_remaining.
//     - When both pools are empty: transition to TalentChoices phase.
//   TalentChoices phase:
//     - Each level-up: owed_choices += 1; ProgressionPlugin drains the backlog through the
//       TalentPicker overlay (one offer at a time so uniqueness reflects each acquisition).
//
// ThroneRoom rewards (Phase 7) reuse the same pending_offer / TalentPicker flow with a
// Rare-or-above rarity floor.

use crate::ability::assets::AbilityId;
use crate::talent::offer::TalentOffer;
use bevy::prelude::*;

/// Standalone Resource in Phase 2; stored inline in RunState from Phase 7 (serialized, Phase 8).
#[derive(Resource, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LevelUpFlowState {
    pub phase: LevelUpPhase,
    /// Shuffled at run start. Draw from the front (`remove(0)`) to pop without replacement.
    pub band_2_3_remaining: Vec<AbilityId>,
    pub band_4_6_remaining: Vec<AbilityId>,
    /// The offer currently shown in the TalentPicker overlay. Generated lazily by the picker.
    pub pending_offer: Option<TalentOffer>,
    /// Talent choices the player still owes (accumulates on TalentChoices-phase level-ups;
    /// drained one at a time by the picker). Lets multiple level-ups in one frame queue offers.
    pub owed_choices: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LevelUpPhase {
    /// Levels 2–6: unlock core abilities from band pools.
    AbilityUnlock,
    /// All core abilities unlocked: subsequent levels offer talent choices.
    TalentChoices,
}

impl LevelUpFlowState {
    /// Initialize from the hero's band pools at run start. Pools should already be shuffled by
    /// the caller (which owns the RunRng). Starts in the AbilityUnlock phase with no backlog.
    pub fn new(band_2_3: Vec<AbilityId>, band_4_6: Vec<AbilityId>) -> Self {
        Self {
            phase: LevelUpPhase::AbilityUnlock,
            band_2_3_remaining: band_2_3,
            band_4_6_remaining: band_4_6,
            pending_offer: None,
            owed_choices: 0,
        }
    }

    /// Draws the next band ability (2/3 pool first, then 4/6). Flips to TalentChoices once both
    /// pools are empty. Returns None if there was nothing left to draw.
    pub fn next_unlock(&mut self) -> Option<AbilityId> {
        let popped = if !self.band_2_3_remaining.is_empty() {
            Some(self.band_2_3_remaining.remove(0))
        } else if !self.band_4_6_remaining.is_empty() {
            Some(self.band_4_6_remaining.remove(0))
        } else {
            None
        };
        if self.band_2_3_remaining.is_empty() && self.band_4_6_remaining.is_empty() {
            self.phase = LevelUpPhase::TalentChoices;
        }
        popped
    }

    /// Records that a level-up in the TalentChoices phase owes the player a talent choice.
    pub fn record_talent_level(&mut self) {
        self.owed_choices += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_drain_then_flip_to_talent_choices() {
        // BDK shapes: 2 in the 2/3 band, 3 in the 4/6 band.
        let mut flow = LevelUpFlowState::new(
            vec!["blood_boil".into(), "heart_strike".into()],
            vec!["abomination_limb".into(), "purgatory".into(), "amz".into()],
        );
        assert_eq!(flow.phase, LevelUpPhase::AbilityUnlock);

        // Five unlocks (L2–L6) drain both pools.
        let mut unlocked = Vec::new();
        for _ in 0..5 {
            assert_eq!(flow.phase, LevelUpPhase::AbilityUnlock);
            unlocked.push(flow.next_unlock().expect("ability available"));
        }
        assert_eq!(unlocked.len(), 5);
        assert_eq!(flow.phase, LevelUpPhase::TalentChoices, "flips after the last band draw");

        // Further level-ups owe talent choices instead of unlocks.
        assert!(flow.next_unlock().is_none());
        flow.record_talent_level();
        flow.record_talent_level();
        assert_eq!(flow.owed_choices, 2);
    }

    #[test]
    fn twenty_third_band_pool_drawn_before_four_six() {
        let mut flow = LevelUpFlowState::new(vec!["a".into(), "b".into()], vec!["c".into()]);
        assert_eq!(flow.next_unlock().as_deref(), Some("a"));
        assert_eq!(flow.next_unlock().as_deref(), Some("b"));
        assert_eq!(flow.next_unlock().as_deref(), Some("c"));
        assert_eq!(flow.phase, LevelUpPhase::TalentChoices);
    }
}
