// Status effect lifecycle: application, ticking, and expiry.
//
// Three responsibilities:
//   1. apply_status_effects — consumes ApplyStatusEvent, spawns StatusEffectInstance child entities.
//      Respects StackingRule: RefreshOnReapply resets the existing timer; StackCapped(n) only
//      spawns if the current stack count < n; StackUnlimited always spawns.
//   2. tick_status_effects — advances timers, fires on_tick_hooks (usually emits DamageEvent
//      for DoT effects like bleed/blaze), despawns expired instances.
//   3. Runs in StatusSet (after CombatSet::Apply, before CombatSet::Death).

use bevy::prelude::*;
use crate::status::assets::StatusEffectDef;
use crate::status::components::{ApplyStatusEvent, RemoveStatusEvent, StatusEffectInstance};

/// TODO(Phase 3): implement.
/// Consumes ApplyStatusEvent, spawns child entities on the target respecting StackingRule.
pub fn apply_status_effects(
    mut _events: EventReader<ApplyStatusEvent>,
    _effect_defs: Res<Assets<StatusEffectDef>>,
    mut _commands: Commands,
    _existing: Query<(&StatusEffectInstance, &Parent)>,
) {
    todo!("Phase 3")
}

/// TODO(Phase 3): implement.
/// Advances timers; fires DoT damage via DamageEvent; despawns expired effects.
pub fn tick_status_effects(
    _time: Res<Time>,
    mut _instances: Query<(Entity, &mut StatusEffectInstance, &Parent)>,
    mut _commands: Commands,
    // + EventWriter<DamageEvent> for DoT effects
) {
    todo!("Phase 3")
}

/// Consumes RemoveStatusEvent, despawns all matching StatusEffectInstance children.
pub fn remove_status_effects(
    mut _events: EventReader<RemoveStatusEvent>,
    mut _commands: Commands,
    _instances: Query<(Entity, &StatusEffectInstance, &Parent)>,
) {
    todo!("Phase 3")
}
