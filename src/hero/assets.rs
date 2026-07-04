// HeroDef — one file per class, loaded from assets/heroes/<id>.ron.
//
// NOTE: All class names, ability names referenced here are WORKING NAMES. The `id` field
// is the stable internal key. `display_name` is what players see on the character select screen.
//
// Stance system:
//   - Classes with has_stance == false (Death Knight, Paladin) have no Q binding.
//     The stance_slots list contains a single entry with stance == "default".
//   - Classes with has_stance == true (Druid, Mage) have two entries (one per form).
//     Q triggers handle_stance_swap which swaps ActiveStance and fires the stance-swap ability.
//
// Interactions:
//   - hero/systems/input_slot.rs reads stance_slots to resolve InputSlot → AbilityId.
//   - hero/systems/stance.rs reads stance_a / stance_b for the swap animation cue.
//   - progression/systems/level_up.rs reads band_2_3_pool / band_4_6_pool.
//   - progression/systems/offer.rs reads class_passive_pool for talent offer generation.

use bevy::prelude::*;
use crate::ability::assets::AbilityId;
use crate::talent::assets::TalentId;

pub type HeroId = String;
pub type StanceId = String;

/// Loaded from assets/heroes/<id>.ron.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct HeroDef {
    pub id: HeroId,
    pub display_name: String,
    pub base_stats: HeroBaseStats,
    pub resource_model: ResourceModel,
    /// true for Druid and Mage (Q swaps stance); false for Death Knight and Paladin.
    pub has_stance: bool,
    /// Only meaningful when has_stance == true.
    pub stance_a: Option<StanceId>,
    pub stance_b: Option<StanceId>,
    /// Always unlocked at level 1, regardless of band pools.
    pub level_1_abilities: Vec<AbilityId>,
    /// Draw one at each of levels 2 and 3 (without replacement).
    pub band_2_3_pool: Vec<AbilityId>,
    /// Draw one at each of levels 4, 5, and 6 (without replacement, from this pool).
    pub band_4_6_pool: Vec<AbilityId>,
    /// Class-specific passive talents available in the offer pool once TalentChoices phase begins.
    pub class_passive_pool: Vec<TalentId>,
    /// Maps each stance to its InputSlot bindings.
    /// Non-stance heroes have a single entry: stance == "default".
    pub stance_slots: Vec<StanceSlotMapping>,
}

#[derive(Debug, Clone)]
pub struct HeroBaseStats {
    pub max_health: f32,
    pub move_speed: f32,
}

/// How the class interacts with resources (mana, health, charges).
#[derive(Debug, Clone)]
pub enum ResourceModel {
    /// No secondary resource bar. Standard health-only class (Druid, Mage, Paladin prototype).
    None,
    /// Health IS the gameplay resource — some abilities cost health or scale with missing health.
    /// No secondary bar; the existing Health component is the resource.
    HealthBased,
}

/// Binds InputSlots to AbilityIds for one stance.
/// Slots without an ability (e.g. StanceSwap for non-stance heroes) are absent from the map.
#[derive(Debug, Clone)]
pub struct StanceSlotMapping {
    pub stance: StanceId,
    pub basic: Option<AbilityId>,
    pub special: Option<AbilityId>,
    pub movement: Option<AbilityId>,
    // StanceSwap is handled separately by hero/systems/stance.rs, not as a normal ability.
}
