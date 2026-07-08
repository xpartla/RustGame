// Handles Q press for stance-swapping heroes (Mage; later Druid).
// No-op for non-stance heroes (Death Knight, Paladin) — their HeroDef.has_stance == false.
//
// On Q press (Phase 4):
//   1. Return early if HeroDef.has_stance == false.
//   2. Flip ActiveStance between stance_a and stance_b.
//   3. Apply the *entered* stance's `swap_effect` status to the caster, if any:
//      - Mage: entering Fire grants "boots_of_fire" (move-speed buff). Emitted through the normal
//        status-apply pipeline (ApplyStatusEvent), so no bespoke system is needed.
//   3b. Grant the *entered* stance's `swap_shield_amount` as a real Absorb shield, if any (Phase
//       9.5) — Mage: entering Ice grants Ice Barrier, via the normal GainShieldEvent pipeline
//       (the same primitive Flash of Light's overheal->shield talent uses).
//
// The stance-swap "ability fires" of heavier classes (Druid Scratch/Roots on swap) are deferred;
// the focused Phase-4 slice models the swap effect as a self-applied status. StanceGate on
// AbilityInstances stays inert — input resolution reads ActiveStance directly (input_slot.rs).
//
// Runs before CombatSet::Damage.

use bevy::prelude::*;
use crate::ability::components::TriggerAbilityEvent;
use crate::core::components::AbilitiesSuppressed;
use crate::core::events::GainShieldEvent;
use crate::hero::assets::{HeroDef, HeroLibrary};
use crate::hero::components::{ActiveStance, HeroIdentity};
use crate::status::components::ApplyStatusEvent;

pub fn handle_stance_swap(
    kb: Res<ButtonInput<KeyCode>>,
    // A suppressed (stunned) player cannot stance-swap — excluded from the query.
    mut player: Query<(Entity, &HeroIdentity, &mut ActiveStance), Without<AbilitiesSuppressed>>,
    hero_library: Res<HeroLibrary>,
    hero_defs: Res<Assets<HeroDef>>,
    mut apply_status: EventWriter<ApplyStatusEvent>,
    mut gain_shield: EventWriter<GainShieldEvent>,
    mut trigger_ability: EventWriter<TriggerAbilityEvent>,
) {
    if !kb.just_pressed(KeyCode::KeyQ) {
        return;
    }
    for (owner, hero_id, mut stance) in &mut player {
        let Some(handle) = hero_library.get(&hero_id.0) else { continue };
        let Some(hero_def) = hero_defs.get(handle) else { continue };
        if !hero_def.has_stance {
            continue;
        }
        let (Some(a), Some(b)) = (hero_def.stance_a.as_ref(), hero_def.stance_b.as_ref()) else {
            continue; // has_stance but stances unset — malformed def; skip defensively.
        };
        // Toggle to the other stance (default to stance_a from any non-b stance).
        let entered = if stance.0 == *a { b } else { a };
        stance.0 = entered.clone();
        // Apply the entered stance's on-swap effect to the caster, if it declares one.
        if let Some(mapping) = hero_def.stance_slots.iter().find(|m| &m.stance == entered) {
            if let Some(effect_id) = &mapping.swap_effect {
                apply_status.write(ApplyStatusEvent {
                    target: owner,
                    source: owner,
                    effect_id: effect_id.clone(),
                    stacks: 1,
                });
            }
            if let Some(amount) = mapping.swap_shield_amount {
                gain_shield.write(GainShieldEvent { target: owner, amount });
            }
            // Phase 9.4 — the Druid: entering a stance also casts its own Basic ability
            // (Scratch on -> Animal, Roots on -> Human). Just a normal TriggerAbilityEvent, so it
            // respects the ability's own cooldown/aim gate like any other cast.
            if mapping.cast_on_enter {
                if let Some(ability_id) = &mapping.basic {
                    trigger_ability.write(TriggerAbilityEvent { ability_id: ability_id.clone(), owner });
                }
            }
        }
    }
}
