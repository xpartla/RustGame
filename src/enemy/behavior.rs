// AiBehaviorRegistry — the AI equivalent of BehaviorRegistry for player abilities.
//
// Each enemy type declares an ai_behavior ID in its EnemyDef. At runtime, the enemy
// update system looks up the ID in AiBehaviorRegistry and calls update() each frame.
//
// Built-in AI behaviors:
//   "melee_chaser"   — flow-field follower (already implemented in enemy/systems/follow_flow_field.rs)
//   "ranged_caster"  — stop at attack range, select target, cast ability
//   "stationary"     — does not move; casts abilities in patterns
//   "boss"           — multi-phase; placeholder until boss kits are designed (Phase 9)
//
// Adding a new AI behavior:
//   1. Implement EnemyAiHook for a unit struct.
//   2. Register it: registry.register("my_behavior", MyAi).
//   3. Set ai_behavior: "my_behavior" in the enemy's RON file.
//
// EnemyAiContext is passed each frame to the active hook. It provides spatial and
// entity data without requiring &mut World access (same constraint as AbilityContext).

use bevy::prelude::*;
use std::collections::HashMap;
use crate::enemy::assets::AiBehaviorId;

/// Context passed to AI hooks each frame. Provides what AI needs to make decisions.
/// TODO(Phase 5): expand with flow field reference, player position, nearby entities.
pub struct EnemyAiContext<'w> {
    pub entity: Entity,
    _phantom: std::marker::PhantomData<&'w ()>,
}

/// Governs one enemy type's movement and targeting decisions.
pub trait EnemyAiHook: Send + Sync + 'static {
    fn update(&self, ctx: &mut EnemyAiContext<'_>);
}

/// Resource: maps AiBehaviorId → boxed AI hook. Populated at plugin build time.
#[derive(Resource, Default)]
pub struct AiBehaviorRegistry {
    behaviors: HashMap<AiBehaviorId, Box<dyn EnemyAiHook>>,
}

impl AiBehaviorRegistry {
    pub fn register(&mut self, id: impl Into<AiBehaviorId>, hook: impl EnemyAiHook) {
        self.behaviors.insert(id.into(), Box::new(hook));
    }

    pub fn get(&self, id: &str) -> Option<&dyn EnemyAiHook> {
        self.behaviors.get(id).map(|h| h.as_ref())
    }
}

// ── Built-in AI implementations (stubs) ────────────────────────────────────────────────────

/// "melee_chaser" — wraps the existing flow-field following logic.
/// In Phase 5, port follow_flow_field.rs into this hook.
pub struct MeleeChaser;
impl EnemyAiHook for MeleeChaser {
    fn update(&self, _ctx: &mut EnemyAiContext<'_>) {
        todo!("Phase 5: delegate to flow field following + contact attack")
    }
}

/// "ranged_caster" — stops at max ability range, selects a target, fires.
pub struct RangedCaster;
impl EnemyAiHook for RangedCaster {
    fn update(&self, _ctx: &mut EnemyAiContext<'_>) {
        todo!("Phase 5")
    }
}

/// "stationary" — does not move; casts abilities on cooldown timers.
pub struct Stationary;
impl EnemyAiHook for Stationary {
    fn update(&self, _ctx: &mut EnemyAiContext<'_>) {
        todo!("Phase 5")
    }
}
