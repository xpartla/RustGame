// Handles Q press for stance-swapping heroes (Druid, Mage).
// No-op for non-stance heroes (Death Knight, Paladin) — their HeroDef.has_stance == false.
//
// On Q press:
//   1. Check if HeroDef.has_stance == true; return early if not.
//   2. Flip ActiveStance between stance_a and stance_b.
//   3. Determine which ability fires on the swap:
//      - Druid: animal→human fires Roots; human→animal fires Scratch
//      - Mage:  fire→ice gains ice barrier; ice→fire gains boots of fire
//      These are treated as immediate ability fires — emit TriggerAbilityEvent for the
//      stance-swap ability defined in HeroDef (a special slot not in the normal slot map).
//   4. Update AbilityInstance StanceGate components to reflect new active stance.
//
// Runs before CombatSet::Damage.

use bevy::prelude::*;
use crate::hero::assets::HeroDef;
use crate::hero::components::{ActiveStance, HeroIdentity};

/// TODO(Phase 4): implement.
pub fn handle_stance_swap(
    _kb: Res<ButtonInput<KeyCode>>,
    _player: Query<(Entity, &HeroIdentity, &mut ActiveStance)>,
    _hero_defs: Res<Assets<HeroDef>>,
) {
    todo!("Phase 4")
}
