// Per-entity status effect state.
//
// Each active effect instance is a separate top-level entity carrying `StatusEffectInstance`.
// The target it afflicts is stored directly in the `target` field (mirroring how AbilityInstance
// stores `owner` — no Bevy hierarchy join needed to query "does entity X have effect Y?"). This
// enables: multiple instances of the same effect (bleed stacks) with independent timers, and
// cheap iteration. Instances whose target has despawned are reaped by `despawn_orphaned_status`.
//
// The ApplyStatusEvent is the public interface for applying effects from abilities
// (EffectSpec::ApplyStatus → ApplyStatusEvent). status/systems/apply.rs spawns instances;
// tick.rs / remove.rs / cross_interact.rs advance and clear them.

use bevy::prelude::*;
use crate::status::assets::StatusEffectId;

/// One active status effect afflicting `target`, applied by `source`.
#[derive(Component, Debug, Clone)]
pub struct StatusEffectInstance {
    pub def_id: StatusEffectId,
    /// The entity this effect is afflicting (query key; no hierarchy).
    pub target: Entity,
    /// Entity that applied this effect. Used for DoT kill credit and damage attribution.
    pub source: Entity,
    /// Remaining duration (TimerMode::Once).
    pub timer: Timer,
    /// Tick cadence — `Some` iff the effect's StatusEffectDef has a `tick` (TimerMode::Repeating).
    pub tick_timer: Option<Timer>,
}

/// Event: request to apply a status effect to a target entity.
/// Emitted by ability execution (EffectSpec::ApplyStatus) and by hooks; consumed by
/// status/systems/apply.rs.
#[derive(Event, Debug)]
pub struct ApplyStatusEvent {
    pub target: Entity,
    pub source: Entity,
    pub effect_id: StatusEffectId,
    /// How many stacks to apply (usually 1; > 1 for effects that apply multiple stacks in one hit).
    pub stacks: u8,
}

/// Event: request to remove all instances of a status effect from a target.
/// Emitted by status/systems/cross_interact.rs (element cancellation) and by talent hooks that
/// consume effects.
#[derive(Event, Debug)]
pub struct RemoveStatusEvent {
    pub target: Entity,
    pub effect_id: StatusEffectId,
}
