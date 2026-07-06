// Talent offer generation — samples the eligible pool respecting all uniqueness constraints.
//
// Called from:
//   - progression/systems/level_up.rs for normal level-up talent choices
//   - progression/systems/offer.rs (ThroneRoom rewards, Phase 7) with a rarity floor
//   - talent/systems/merchant.rs for the 3-for-1 trade-up (Phase 8)
//
// The pool sampled from is: all talents in the player's unlocked abilities' talent_pools
// + HeroDef.class_passive_pool + general passive pool. The caller (progression) builds this
// list of ids; generate_offer resolves each to its TalentDef and filters by uniqueness + rarity.
//
// Uniqueness checks (per TalentDef.uniqueness), see is_eligible:
//   None                     — always eligible
//   Stack(n)                 — eligible if acquired count < n
//   Exclusive                — eligible if count == 0
//   MutuallyExcludes(other)  — eligible if `other` not in AcquiredTalents
//
// Uses RunRng (not thread_rng) so offers are seed-deterministic.

use crate::run::rng::RunRng;
use crate::talent::assets::{TalentDef, TalentId, TalentLibrary, TalentRarity, UniquenessConstraint};
use crate::talent::components::AcquiredTalents;
use bevy::asset::Assets;
use rand::seq::SliceRandom;

/// A single talent offer presented to the player.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TalentOffer {
    /// Up to 3 options. Fewer than 3 if the eligible pool is exhausted.
    pub options: Vec<TalentId>,
    /// Origin of the offer. Carried for the UI to theme the screen, for merchant/ThroneRoom
    /// flows, and serialized as part of `LevelUpFlowState` (Phase 8 — a save mid-offer restores
    /// the exact offer on resume rather than re-rolling it).
    pub context: OfferContext,
}

/// Where the offer came from — displayed differently in the UI, and sets the rarity floor.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OfferContext {
    /// Normal level-up after all core abilities are unlocked. Any rarity.
    LevelUp,
    /// ThroneRoom reward: always Rare or better.
    ThroneRoom,
    /// Post-special-event: Rare or Epic only. No special-event flow exists yet (architecture-plan
    /// §8.1(10)) — kept for the offer-context shape; never constructed until one lands.
    #[allow(dead_code)]
    SpecialEvent,
    /// Merchant 3-for-1 trade: higher rarity than the traded-in set.
    MerchantTradeUp { min_rarity: TalentRarity },
}

impl OfferContext {
    /// Minimum rarity that may be offered in this context. `None` means any rarity.
    pub fn min_rarity(&self) -> Option<TalentRarity> {
        match self {
            OfferContext::LevelUp => None,
            OfferContext::ThroneRoom => Some(TalentRarity::Rare),
            OfferContext::SpecialEvent => Some(TalentRarity::Rare),
            OfferContext::MerchantTradeUp { min_rarity } => Some(min_rarity.clone()),
        }
    }
}

/// Generates a talent offer for the player.
///
/// `eligible_talent_ids` — the full pool of TalentIds the player could potentially receive
///   (union of all unlocked ability talent_pools + class passives + general passives). The caller
///   builds this; ids with no loaded TalentDef are skipped so unimplemented content self-filters.
pub fn generate_offer(
    context: OfferContext,
    eligible_talent_ids: &[TalentId],
    acquired: &AcquiredTalents,
    talent_defs: &Assets<TalentDef>,
    library: &TalentLibrary,
    rng: &mut RunRng,
) -> TalentOffer {
    let min_rarity = context.min_rarity();

    // Resolve ids → defs, drop duplicates, keep only currently-eligible talents.
    let mut candidates: Vec<TalentId> = Vec::new();
    for id in eligible_talent_ids {
        if candidates.iter().any(|c| c == id) {
            continue; // an id can appear in several pools; offer it at most once
        }
        let Some(def) = library.get(id).and_then(|h| talent_defs.get(h)) else {
            continue; // not loaded / no RON file yet
        };
        if is_eligible(def, acquired, min_rarity.as_ref()) {
            candidates.push(id.clone());
        }
    }

    // Sample up to 3 distinct options using the seeded RNG.
    let options: Vec<TalentId> = candidates
        .choose_multiple(rng.rng(), 3)
        .cloned()
        .collect();

    TalentOffer { options, context }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::talent::assets::{ModOp, StatModifier, TalentEffect};

    fn def(id: &str, rarity: TalentRarity, uniqueness: UniquenessConstraint) -> TalentDef {
        TalentDef {
            id: id.to_string(),
            display_name: id.to_string(),
            ability_scope: Some("death_strike".to_string()),
            rarity,
            uniqueness,
            effect: TalentEffect::Modifier(StatModifier {
                stat: "damage".to_string(),
                op: ModOp::MultiplyAdd(0.2),
            }),
        }
    }

    #[test]
    fn stack_eligible_until_cap() {
        let d = def("t", TalentRarity::Common, UniquenessConstraint::Stack(3));
        let mut acq = AcquiredTalents::default();
        assert!(is_eligible(&d, &acq, None));
        acq.add("t".to_string());
        acq.add("t".to_string());
        assert!(is_eligible(&d, &acq, None), "2 < 3 still eligible");
        acq.add("t".to_string());
        assert!(!is_eligible(&d, &acq, None), "3 == cap, no longer eligible");
    }

    #[test]
    fn exclusive_offered_once() {
        let d = def("t", TalentRarity::Epic, UniquenessConstraint::Exclusive);
        let mut acq = AcquiredTalents::default();
        assert!(is_eligible(&d, &acq, None));
        acq.add("t".to_string());
        assert!(!is_eligible(&d, &acq, None));
    }

    #[test]
    fn mutually_excludes_blocks_when_other_present() {
        let d = def("fiery_ent", TalentRarity::Epic,
            UniquenessConstraint::MutuallyExcludes("earth_ent".to_string()));
        let mut acq = AcquiredTalents::default();
        assert!(is_eligible(&d, &acq, None));
        acq.add("earth_ent".to_string());
        assert!(!is_eligible(&d, &acq, None));
    }

    #[test]
    fn rarity_floor_filters_common() {
        let common = def("c", TalentRarity::Common, UniquenessConstraint::None);
        let rare = def("r", TalentRarity::Rare, UniquenessConstraint::None);
        let acq = AcquiredTalents::default();
        assert!(!is_eligible(&common, &acq, Some(&TalentRarity::Rare)));
        assert!(is_eligible(&rare, &acq, Some(&TalentRarity::Rare)));
    }
}
