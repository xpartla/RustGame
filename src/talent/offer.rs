// Talent offer generation — samples the eligible pool respecting all uniqueness constraints.
//
// Called from:
//   - progression/systems/level_up.rs for normal level-up talent choices
//   - world/systems (or encounter system) for ThroneRoom rewards (RarityFilter::RareOrAbove)
//   - talent/systems/merchant.rs for the 3-for-1 trade-up
//
// The pool sampled from is: all talents in the player's unlocked abilities' talent_pools
// + HeroDef.class_passive_pool + general passive pool. Each talent in the pool is checked
// against uniqueness constraints before being eligible for offer.
//
// Uniqueness checks (per TalentDef.uniqueness):
//   None                     — always eligible
//   Stack(n)                 — eligible if acquired count < n
//   Exclusive                — eligible if count == 0
//   MutuallyExcludes(other)  — eligible if `other` not in AcquiredTalents
//
// Uses RunRng (not thread_rng) so offers are seed-deterministic.

use crate::run::rng::RunRng;
use crate::talent::assets::{TalentDef, TalentId, TalentRarity, UniquenessConstraint};
use crate::talent::components::AcquiredTalents;
use bevy::asset::Assets;

/// A single talent offer presented to the player.
#[derive(Debug, Clone)]
pub struct TalentOffer {
    /// Up to 3 options. Fewer than 3 if the eligible pool is exhausted.
    pub options: Vec<TalentId>,
    pub context: OfferContext,
}

/// Where the offer came from — displayed differently in the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfferContext {
    /// Normal level-up after all core abilities are unlocked. Any rarity.
    LevelUp,
    /// ThroneRoom reward: always Rare or better.
    ThroneRoom,
    /// Post-special-event: Rare or Epic only.
    SpecialEvent,
    /// Merchant 3-for-1 trade: higher rarity than the traded-in set.
    MerchantTradeUp { min_rarity: TalentRarity },
}

/// Generates a talent offer for the player.
///
/// `eligible_talent_ids` — the full pool of TalentIds the player could potentially receive
///   (union of all unlocked ability talent_pools + class passives + general passives).
///   The caller builds this from AbilityDef.talent_pool + HeroDef.class_passive_pool.
///
/// TODO(Phase 2): implement.
pub fn generate_offer(
    context: OfferContext,
    eligible_talent_ids: &[TalentId],
    acquired: &AcquiredTalents,
    talent_defs: &Assets<TalentDef>,
    rng: &mut RunRng,
) -> TalentOffer {
    let _ = (eligible_talent_ids, acquired, talent_defs, rng);
    todo!("Phase 2: sample up to 3 unique talents from eligible_talent_ids, \
           filtered by uniqueness constraints and rarity filter from context")
}

/// Checks whether a talent is eligible to be offered given current acquired talents.
pub fn is_eligible(
    talent: &TalentDef,
    acquired: &AcquiredTalents,
    min_rarity: Option<&TalentRarity>,
) -> bool {
    // Rarity filter
    if let Some(min) = min_rarity {
        if &talent.rarity < min {
            return false;
        }
    }

    // Uniqueness
    match &talent.uniqueness {
        UniquenessConstraint::None => true,
        UniquenessConstraint::Stack(n) => acquired.count_of(&talent.id) < *n,
        UniquenessConstraint::Exclusive => !acquired.has(&talent.id),
        UniquenessConstraint::MutuallyExcludes(other) => !acquired.has(other),
    }
}
