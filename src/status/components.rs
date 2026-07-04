// Per-entity status effect state.
//
// Each active effect instance is a child entity of the target (player or enemy).
// This enables: multiple instances of the same effect (bleed stacks), independent timers,
// and cheap ECS queries ("does entity X have effect Y?").
//
// Querying for a specific effect:
//   Query<(&StatusEffectInstance, &Parent)> where instance.def_id == "frostbite"
//   Filter by Parent to narrow to a specific entity.
//
// The ApplyStatusEvent is the public interface for applying effects from abilities.
// status/systems/tick.rs handles the actual spawning and removal.

use bevy::prelude::*;
use crate::status::assets::StatusEffectId;

/// Marks the effect as coming from a specific source (for kill credit and damage attribution).
#[derive(Component, Debug, Clone)]
pub struct StatusEffectInstance {
    pub def_id: StatusEffectId,
    /// Entity that applied this effect. Used for kill credit (DoT kills) and leech attribution.
    pub source: Entity,
    pub timer: Timer,
    /// Tick timer — only meaningful if StatusEffectDef.tick_interval_secs is Some.
    pub tick_timer: Option<Timer>,
}

/// Event: request to apply a status effect to a target entity.
/// Emitted by ability behaviors and hooks; consumed by status/systems/tick.rs.
#[derive(Event, Debug)]
pub struct ApplyStatusEvent {
    pub target: Entity,
    pub source: Entity,
    pub effect_id: StatusEffectId,
    /// How many stacks to apply (usually 1; > 1 for effects that apply multiple stacks in one hit).
    pub stacks: u8,
}

/// Event: request to remove all instances of a status effect from a target.
/// Emitted by status/systems/cross_interact.rs and by talent hooks that consume effects.
#[derive(Event, Debug)]
pub struct RemoveStatusEvent {
    pub target: Entity,
    pub effect_id: StatusEffectId,
}
