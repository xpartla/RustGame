// Hero runtime state on the player entity.
//
// These components are set at run start (when the hero is selected) and remain for the run's
// lifetime. HeroId drives asset lookup; ActiveStance drives input-slot resolution each frame.
//
// Interactions:
//   - hero/systems/input_slot.rs reads HeroId + ActiveStance to look up the correct AbilityId.
//   - hero/systems/stance.rs reads/writes ActiveStance on Q press.
//   - ui/ reads HeroId to display the correct character art and ability tooltips.

use bevy::prelude::*;
use crate::hero::assets::{HeroId, StanceId};

/// The class the player spawns as by default (until a character-select flow exists — later
/// phase). The Death Knight keeps the prototype's kit, so the golden-master baseline is stable.
pub const DEFAULT_HERO_ID: &str = "blood_death_knight";

/// Which hero class this player entity represents. Immutable for the run's lifetime.
#[derive(Component, Debug, Clone)]
pub struct HeroIdentity(pub HeroId);

/// The currently active form/stance. For non-stance heroes this is always "default".
/// Updated by hero/systems/stance.rs on Q press.
#[derive(Component, Debug, Clone)]
pub struct ActiveStance(pub StanceId);

impl Default for ActiveStance {
    fn default() -> Self {
        Self("default".to_string())
    }
}

/// The four input slots. Mapped to AbilityIds via HeroDef.stance_slots in input_slot.rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSlot {
    /// Left click (or LMB equivalent).
    Basic,
    /// Right click (or RMB equivalent).
    Special,
    /// Shift / Space — dash or movement ability.
    Movement,
    /// Q — stance swap. No-op for non-stance heroes.
    StanceSwap,
}

/// Optional class resource tracking, read directly by the HUD's class-resource bar
/// (ui/screens/hud.rs::update_class_resource). Nothing inserts this by hand for a `Charges`-backed
/// hero — `hero::systems::resource::sync_charges_to_class_resource` mirrors `Charges` into it, so
/// the bar lights up with zero HUD work the moment a hero's kit starts using charges.
#[derive(Component, Debug, Default)]
pub struct ClassResource {
    pub current: f32,
    pub max: f32,
}

/// A capped integer resource bar (Phase 9.1 primitive; `ResourceModel::Charges` — hero/assets.rs).
/// Mage frost charges and Druid enhanced/combo charges are the first consumers (Phase 9.4/9.5); no
/// shipped hero grants one yet. Transient — never serialized into `RunState` (a charge count is
/// mid-encounter state, reset like the rest of live combat state on resume).
#[derive(Component, Debug, Clone, Copy)]
pub struct Charges {
    pub current: u32,
    pub max: u32,
}

impl Charges {
    /// A fresh, empty charge bar capped at `max`.
    pub fn new(max: u32) -> Self {
        Self { current: 0, max }
    }

    /// Adds `n` charges, capped at `max`.
    pub fn gain(&mut self, n: u32) {
        self.current = (self.current + n).min(self.max);
    }

    /// Consumes every charge, returning how many were spent.
    pub fn spend_all(&mut self) -> u32 {
        std::mem::replace(&mut self.current, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_caps_at_max() {
        let mut charges = Charges::new(3);
        charges.gain(2);
        assert_eq!(charges.current, 2);
        charges.gain(5); // would overflow past max
        assert_eq!(charges.current, 3, "gain never exceeds max");
    }

    #[test]
    fn spend_all_returns_the_spent_amount_and_resets_to_zero() {
        let mut charges = Charges::new(5);
        charges.gain(4);
        let spent = charges.spend_all();
        assert_eq!(spent, 4);
        assert_eq!(charges.current, 0);
        // Spending an empty bar returns 0, not an underflow panic.
        assert_eq!(charges.spend_all(), 0);
    }
}
