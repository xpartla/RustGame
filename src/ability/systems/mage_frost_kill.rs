// Frost-kill class passives (Phase 9.5 — the Mage's "Frostbite" passive section's two
// kill-reactive talents, `ability_scope: None`). Mirrors `ability::systems::bone_shield`'s shape
// exactly: read `Health`/`LastHitBy` on a dying `Enemy` in `CombatSet::Death`, gated on the
// killer's `ActiveHooks` — the one extra read is whether the dying enemy carried a "frostbite"
// `StatusEffectInstance` at the moment of death. Registered `.before(enemy_death)` in
// ability/plugin.rs (not order-agnostic — see that pin's comment for why).
//
// The other three Frostbite/Blaze/Frost-charge talents are deferred (status-magnitude — no
// primitive rescales a `StatusEffectDef`'s own fields; see CHANGELOG "Phase 9.5").

use bevy::prelude::*;
use crate::core::components::{Health, LastHitBy};
use crate::core::events::HealEvent;
use crate::enemy::components::Enemy;
use crate::hero::components::Charges;
use crate::status::components::StatusEffectInstance;
use crate::talent::components::ActiveHooks;

/// Mechanics: "Heal X% of your max health after killing an enemy affected by frostbite."
const FROSTBITTEN_KILL_HEAL_FRACTION: f32 = 0.05;

/// "Gain a frost charge if an enemy affected by frostbite dies" (`mage_passive_frost_charge_on_
/// frostbitten_kill_rare`).
pub fn frost_charge_on_frostbitten_kill(
    dying: Query<(Entity, &Health, &LastHitBy), With<Enemy>>,
    statuses: Query<&StatusEffectInstance>,
    mut killers: Query<(&ActiveHooks, &mut Charges)>,
) {
    for (entity, health, last_hit_by) in &dying {
        if health.current > 0.0 {
            continue;
        }
        if !statuses.iter().any(|s| s.target == entity && s.def_id == "frostbite") {
            continue;
        }
        let Ok((hooks, mut charges)) = killers.get_mut(last_hit_by.0) else { continue };
        if !hooks.contains("mage_frost_charge_on_frostbitten_kill") {
            continue;
        }
        charges.gain(1);
    }
}

/// "Heal X% of your max health after killing an enemy affected by frostbite"
/// (`mage_passive_frostbitten_kill_heal_epic`).
pub fn heal_on_frostbitten_kill(
    mut heal_events: EventWriter<HealEvent>,
    dying: Query<(Entity, &Health, &LastHitBy), With<Enemy>>,
    statuses: Query<&StatusEffectInstance>,
    killers: Query<(&ActiveHooks, &Health)>,
) {
    for (entity, health, last_hit_by) in &dying {
        if health.current > 0.0 {
            continue;
        }
        if !statuses.iter().any(|s| s.target == entity && s.def_id == "frostbite") {
            continue;
        }
        let Ok((hooks, killer_health)) = killers.get(last_hit_by.0) else { continue };
        if !hooks.contains("mage_frostbitten_kill_heal") {
            continue;
        }
        heal_events.write(HealEvent {
            target: last_hit_by.0,
            amount: killer_health.max * FROSTBITTEN_KILL_HEAL_FRACTION,
        });
    }
}
