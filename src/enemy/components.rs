use bevy::color::Color;
use bevy::prelude::{Component, Entity, Resource, Timer};
use crate::enemy::assets::AiBehaviorId;

#[derive(Component)]
pub struct Enemy;

/// Marks the designated map boss of an encounter (Phase 7). The `KillMapBoss` objective completes
/// when no `MapBoss` entity remains. Tagged onto the boss spawned from a theme's `map_boss_pool`
/// (Map/KillMapBoss), `boss_pool` (BossRoom), or the act boss (ActBoss); ordinary pack enemies do
/// not carry it.
#[derive(Component)]
pub struct MapBoss;

/// Visual shape for an enemy type (built into a `Mesh2d` at spawn). Sourced from
/// `EnemyDef.appearance.shape` (Phase 5); `Deserialize` so it parses straight from the RON.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub enum EnemyShape {
    Circle,
    Triangle,
    Square,
}

/// Visual identity copied from the `EnemyDef` at spawn. Pure data — the presentation layer
/// (enemy/systems/visuals.rs) reads it to build the Mesh2d/material, so headless simulations
/// never touch render assets.
#[derive(Component, Clone, Copy)]
pub struct EnemyAppearance {
    pub shape: EnemyShape,
    pub radius: f32,
    pub color: Color,
}

#[derive(Resource)]
pub struct EnemySpawner {
    pub timer: Timer,
    pub radius: i32,
}

/// Experience awarded to the killer when this enemy dies. Set from the `EnemyDef` at spawn
/// (scaled by depth via `resolve_enemy_stats`).
#[derive(Component)]
pub struct XpReward(pub u32);

/// Which AI drives an enemy's movement/targeting, set at spawn from `EnemyDef.ai_behavior`
/// (Phase 5). Replaces the scaffold's `AiBehaviorRegistry` trait-object dispatch with a plain
/// component enum: movement AI needs world access (flow field, player position, velocity/facing
/// writes), which the `&mut World`-free hook could not express. A new AI = one variant + one
/// system (the content-extensibility axis is already served by the ability `BehaviorRegistry`).
#[derive(Component, Clone, Copy, Debug)]
pub enum AiBehavior {
    /// Flow-field follower (grunt/runner/brute). Faces its movement direction.
    MeleeChaser,
    /// Approaches to `preferred_range`, stops, and faces the player to fire a ranged ability.
    RangedCaster { preferred_range: f32 },
    /// Does not move; faces the player and casts on cooldown.
    Stationary,
}

impl AiBehavior {
    /// Maps the `EnemyDef.ai_behavior` string to a behavior. Unknown ids fall back to
    /// `MeleeChaser` (the original prototype behavior).
    pub fn from_id(id: &AiBehaviorId, preferred_range: f32) -> Self {
        match id.as_str() {
            "ranged_caster" => AiBehavior::RangedCaster { preferred_range },
            "stationary" => AiBehavior::Stationary,
            _ => AiBehavior::MeleeChaser,
        }
    }
}

/// Marks a Friendly minion as a taunt source (Phase 9.4 — Druid's Spawn Ent: "forcing the enemy to
/// attack the Ent instead of you"). Inserted on the minion at spawn (`ability::systems::execute`)
/// when its summon ability resolves a positive `"taunt_radius"` param; absent for every other
/// minion (Companion). Read only by `enemy::systems::taunt::apply_ent_taunt`.
#[derive(Component, Debug, Clone, Copy)]
pub struct Taunt {
    pub radius: f32,
}

/// Marks a Hostile enemy as currently redirected toward a `Taunt` source instead of the player
/// (Phase 9.4). Reconciled every frame by `apply_ent_taunt`; consumed by `enemy_follow_flow_field`,
/// which steers straight-line toward `.0`'s position instead of the flow field while present —
/// `update_enemy_facing` needs no change, since it derives facing from the (now taunt-directed)
/// velocity already. Scoped to `MeleeChaser` enemies only (see `apply_ent_taunt`'s doc comment).
#[derive(Component, Debug, Clone, Copy)]
pub struct Taunted(pub Entity);
