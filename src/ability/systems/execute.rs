// Drives the per-frame ability execution loop.
//
// Two systems (ordered, in CombatSet::Damage):
//   tick_ability_cooldowns  — advances AbilityCooldown.elapsed for all AbilityInstance entities
//   execute_ready_abilities — for each ready ability, resolves params, runs hooks + behavior
//
// Passive abilities (no InputSlot binding) fire automatically when cooldown expires.
// Active abilities fire only when TriggerAbilityEvent arrives for their AbilityId.
//
// Execution order per ability:
//   1. resolve_params() → ResolvedParams
//   2. Pre-hooks (those registered in AbilityDef.hooks with HookPhase::Pre)
//      → only fires if player has ActiveHook(hook_id) component
//   3. BehaviorRegistry.get(behavior_id).execute(ctx, params)
//   4. Post-hooks (HookPhase::Post)
//   5. Reset AbilityCooldown.elapsed = 0, set duration from params("cooldown")

use bevy::prelude::*;

/// TODO(Phase 1): implement.
/// Query signature will be approximately:
///   ability_instances: Query<(Entity, &AbilityInstance, &mut AbilityCooldown, Option<&StanceGate>), With<Parent>>,
///   player: Query<(Entity, &ActiveStance, &AcquiredTalents, &WorldPosition, &Facing), With<Player>>,
///   behavior_registry: Res<BehaviorRegistry>,
///   hook_registry: Res<HookRegistry>,
///   ability_defs: Res<Assets<AbilityDef>>,
///   trigger_events: EventReader<TriggerAbilityEvent>,
///   damage_events: EventWriter<DamageEvent>,
pub fn tick_ability_cooldowns() {
    todo!("Phase 1")
}

pub fn execute_ready_abilities() {
    todo!("Phase 1")
}
