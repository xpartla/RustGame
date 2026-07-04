// Per-ability runtime state attached to each unlocked ability entity.
//
// Each unlocked ability is a separate Bevy entity, parented to the player entity.
// This allows ECS queries to filter by ability type, stance, and cooldown state
// without storing a Vec on the player component.
//
// Spawn: ability/systems/spawn (called from progression/systems/level_up.rs on UnlockAbilityEvent)
// Query: ability/systems/execute.rs reads these to drive cooldowns and execution

use bevy::prelude::*;
use crate::ability::assets::AbilityId;

/// Marker + identity for a runtime ability entity.
/// The entity has this component plus an AbilityCooldown and optionally a StanceGate.
///
/// Phase 1 stores the owner directly rather than using Bevy hierarchy (ChildOf). The plan's
/// "child of the player" framing is honored logically via `owner`; parent/child wiring can be
/// added later without changing the execution query.
#[derive(Component, Debug, Clone)]
pub struct AbilityInstance {
    /// Links back to the AbilityDef asset for behavior ID, base params, and talent pool.
    pub def_id: AbilityId,
    /// The entity that owns/casts this ability (the player, for now).
    pub owner: Entity,
}

/// Tracks remaining cooldown. The execution system fires when elapsed ≥ cooldown param.
/// Cooldown value comes from ResolvedParams("cooldown") each time the ability fires, so
/// talents that reduce cooldown take effect immediately on the next cast.
#[derive(Component, Debug)]
pub struct AbilityCooldown {
    pub elapsed: f32,
    /// Last resolved cooldown duration. Updated from ResolvedParams on each fire.
    pub duration: f32,
}

impl AbilityCooldown {
    pub fn new(duration: f32) -> Self {
        // Start ready (elapsed == duration) so passive abilities fire immediately.
        Self { elapsed: duration, duration }
    }

    pub fn is_ready(&self) -> bool {
        self.elapsed >= self.duration
    }
}

/// Optional stance gate. If present, this ability only executes when the player's
/// ActiveStance matches. Absent = executes in all stances (e.g. passive abilities).
/// Reserved for the stance system (Phase 4).
#[allow(dead_code)]
#[derive(Component, Debug, Clone)]
pub struct StanceGate(pub String); // StanceId

/// Per-ability state storage for behavior hooks that need persistent counters.
/// Example: bone shield kill counter, frost charge count. Reserved for hooks (Phase 2+).
#[allow(dead_code)]
#[derive(Component, Debug, Default)]
pub struct AbilityHookState(pub std::collections::HashMap<String, f32>);

/// Event emitted by hero/systems/input_slot.rs when the player presses an input slot.
/// The ability execution system listens for this and fires the matching AbilityInstance.
#[derive(Event, Debug)]
pub struct TriggerAbilityEvent {
    pub ability_id: AbilityId,
    pub owner: Entity,
}

/// Event emitted when an ability is unlocked — by `grant_level_1_abilities` at spawn (Phase-2
/// stub for HeroDef.level_1_abilities) and by progression/systems/level_up.rs for band unlocks.
/// The ability plugin listens and spawns the AbilityInstance entity (idempotently).
#[derive(Event, Debug)]
pub struct UnlockAbilityEvent {
    pub ability_id: AbilityId,
    pub owner: Entity,
}
