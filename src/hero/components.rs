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

/// Optional class resource tracking (for classes with non-health resources in the future).
/// Currently unused — placeholder for expansion beyond HealthBased / None models.
#[derive(Component, Debug, Default)]
pub struct ClassResource {
    pub current: f32,
    pub max: f32,
}
