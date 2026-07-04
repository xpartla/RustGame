// BehaviorRegistry and HookRegistry — the two open extension points of the ability system.
//
// Adding a new ability shape:
//   1. Implement AbilityBehavior for a unit struct.
//   2. Call registry.register_behavior("my_behavior", MyBehavior) in a plugin.
//   3. Set `behavior: "my_behavior"` in the ability's RON file.
//   No other code changes.
//
// Adding a behavior-rewriting talent:
//   1. Implement AbilityHook for a unit struct.
//   2. Call registry.register_hook("my_hook", MyHook) in a plugin.
//   3. Set `effect: Behavior("my_hook")` in the talent's RON file.
//   4. Add (Pre | Post, "my_hook") to the ability's `hooks` list in its RON file.
//   No other code changes.
//
// Interactions:
//   - ability/systems/execute.rs calls BehaviorRegistry and HookRegistry each frame.
//   - talent/systems/apply.rs inserts/removes ActiveHook components, which gate hook execution.
//   - AbilityContext is built from the player entity's components by execute.rs.

use bevy::prelude::*;
use std::collections::HashMap;
use crate::ability::assets::{BehaviorId, HookId, StatId};

/// Resolved numeric parameters after the talent modifier stack has been applied.
/// Produced by resolve_params() in ability/systems/resolve_params.rs.
#[derive(Debug, Clone)]
pub struct ResolvedParams(pub HashMap<StatId, f32>);

impl ResolvedParams {
    /// Returns the param value, or `default` if the stat is not present.
    pub fn get(&self, stat: &str) -> f32 {
        *self.0.get(stat).unwrap_or(&0.0)
    }
}

/// Context passed to ability behaviors and hooks. Provides what a behavior needs to
/// act (entity identity, spatial data, event writers) without requiring &mut World.
///
/// TODO(Phase 1): Expand with EventWriter<DamageEvent>, EventWriter<SpawnProjectileEvent>,
/// player WorldPosition, Facing, etc. as the first behaviors are implemented.
pub struct AbilityContext<'w> {
    pub owner: Entity,
    // TODO(Phase 1): add position, facing, event writers
    _phantom: std::marker::PhantomData<&'w ()>,
}

/// The base execution logic for one ability shape (melee cone, projectile, zone drop, etc.).
/// Registered once in AbilityPlugin::build; referenced by BehaviorId string from RON.
pub trait AbilityBehavior: Send + Sync + 'static {
    fn execute(&self, ctx: &mut AbilityContext<'_>, params: &ResolvedParams);
}

/// A pre/post execution hook attached to a specific ability by talent acquisition.
/// Fires only when the player has the corresponding ActiveHook component.
pub trait AbilityHook: Send + Sync + 'static {
    fn execute(&self, ctx: &mut AbilityContext<'_>, params: &ResolvedParams);
}

/// Resource: maps BehaviorId → boxed behavior. Populated at plugin build time; read-only at runtime.
#[derive(Resource, Default)]
pub struct BehaviorRegistry {
    behaviors: HashMap<BehaviorId, Box<dyn AbilityBehavior>>,
}

impl BehaviorRegistry {
    pub fn register(&mut self, id: impl Into<BehaviorId>, behavior: impl AbilityBehavior) {
        self.behaviors.insert(id.into(), Box::new(behavior));
    }

    pub fn get(&self, id: &str) -> Option<&dyn AbilityBehavior> {
        self.behaviors.get(id).map(|b| b.as_ref())
    }
}

/// Resource: maps HookId → boxed hook. Populated at plugin build time; read-only at runtime.
#[derive(Resource, Default)]
pub struct HookRegistry {
    hooks: HashMap<HookId, Box<dyn AbilityHook>>,
}

impl HookRegistry {
    pub fn register(&mut self, id: impl Into<HookId>, hook: impl AbilityHook) {
        self.hooks.insert(id.into(), Box::new(hook));
    }

    pub fn get(&self, id: &str) -> Option<&dyn AbilityHook> {
        self.hooks.get(id).map(|h| h.as_ref())
    }
}

// ── Built-in behaviors (stubbed; implement in Phase 1) ─────────────────────────────────────

/// Melee cone: hits all enemies within `range` and within `half_angle` of Facing.
/// Params: "damage", "range", "half_angle", "cooldown", "leech_percent"
pub struct MeleeCone;
impl AbilityBehavior for MeleeCone {
    fn execute(&self, _ctx: &mut AbilityContext<'_>, _params: &ResolvedParams) {
        todo!("Phase 1: MeleeCone — replaces player_arc_attack")
    }
}

/// Travelling projectile: spawns a Projectile entity moving in Facing direction.
/// Params: "damage", "speed", "range", "size", "pierce_count", "cooldown"
pub struct TravellingProjectile;
impl AbilityBehavior for TravellingProjectile {
    fn execute(&self, _ctx: &mut AbilityContext<'_>, _params: &ResolvedParams) {
        todo!("Phase 1: TravellingProjectile")
    }
}

/// Self-centered periodic zone: spawns a PersistentZone at owner position on cooldown.
/// Params: "damage_per_tick", "radius", "duration", "cooldown", "zone_type"
pub struct PeriodicSelfZone;
impl AbilityBehavior for PeriodicSelfZone {
    fn execute(&self, _ctx: &mut AbilityContext<'_>, _params: &ResolvedParams) {
        todo!("Phase 6: PeriodicSelfZone — used by D&D, Consecrated Ground")
    }
}

/// Zone dropped at current position (like Consecrated Ground trail).
/// Params: "damage_per_tick", "radius", "duration", "drop_interval", "zone_type"
pub struct DroppedZone;
impl AbilityBehavior for DroppedZone {
    fn execute(&self, _ctx: &mut AbilityContext<'_>, _params: &ResolvedParams) {
        todo!("Phase 6: DroppedZone")
    }
}

/// Orbiting effect: spawns entities rotating around the owner.
/// Params: "damage", "orbit_radius", "orbit_speed", "count"
pub struct Orbiting;
impl AbilityBehavior for Orbiting {
    fn execute(&self, _ctx: &mut AbilityContext<'_>, _params: &ResolvedParams) {
        todo!("Phase 1: Orbiting — used by Spinning Hammer")
    }
}

/// Leap to target: dashes the owner to the nearest enemy within cursor radius.
/// Params: "damage", "max_range", "cooldown"
pub struct LeapToTarget;
impl AbilityBehavior for LeapToTarget {
    fn execute(&self, _ctx: &mut AbilityContext<'_>, _params: &ResolvedParams) {
        todo!("Phase 1: LeapToTarget — used by Ferocious Bite")
    }
}

/// Channel while moving: fires a multi-tick heal or beam while the player holds the button.
/// Params: "heal_percent", "channel_duration", "cooldown"
pub struct ChannelWhileMoving;
impl AbilityBehavior for ChannelWhileMoving {
    fn execute(&self, _ctx: &mut AbilityContext<'_>, _params: &ResolvedParams) {
        todo!("Phase 1: ChannelWhileMoving — used by Heal, Flash of Light, Frost Impale")
    }
}

/// Summon: spawns a companion entity that uses one of the player's other abilities.
/// Params: "spawn_interval", "duration", "mimicked_ability_id"
pub struct Summon;
impl AbilityBehavior for Summon {
    fn execute(&self, _ctx: &mut AbilityContext<'_>, _params: &ResolvedParams) {
        todo!("Phase 1: Summon — used by Companion (mimics Death Strike)")
    }
}
