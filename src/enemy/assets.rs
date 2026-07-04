// EnemyDef and ThemeDef — data templates for enemy types and map themes.
//
// NOTE: All enemy and theme names are WORKING NAMES. The `id` field is stable.
// Enemy ability kits (EnemyAbilityDef) are TBD — define the interface now,
// fill in content in Phase 9.
//
// Structural similarity to the player system:
//   EnemyDef.abilities → Vec<EnemyAbilityDef> (same BehaviorId + params pattern)
//   EnemyDef.ai_behavior → AiBehaviorId (maps to AiBehaviorRegistry in behavior.rs)
//
// Enemy abilities are simpler than player abilities (no stance, no talent modification)
// but reuse the same BehaviorRegistry execution path where shapes overlap.
//
// Interactions:
//   - enemy/systems/spawner.rs reads EnemyDef to set up components at spawn.
//   - enemy/behavior.rs AiBehaviorRegistry is keyed by ai_behavior field.
//   - world/systems: uses ThemeDef to select enemy pool for each encounter.
//   - ThemeDef is loaded from assets/themes/<id>.ron.

use bevy::prelude::*;
use crate::ability::assets::{BehaviorId, StatId};
use std::collections::HashMap;

pub type EnemyId = String;
pub type ThemeId = String;
pub type AiBehaviorId = String;

/// Loaded from assets/enemies/<id>.ron.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct EnemyDef {
    pub id: EnemyId,
    pub display_name: String,
    pub rarity: EnemyRarity,
    pub base_stats: EnemyBaseStats,
    /// Key into AiBehaviorRegistry — governs targeting and movement decisions.
    pub ai_behavior: AiBehaviorId,
    /// Abilities this enemy can use. Structure mirrors the player ability system.
    /// Content is TBD for all enemy types — define here when designing enemy kits.
    pub abilities: Vec<EnemyAbilityDef>,
    pub xp_value: u32,
    /// ID of a drop table definition (TBD; placeholder string for now).
    pub drop_table: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnemyRarity {
    Common,
    Elite,
    MapBoss,
    ActBoss,
}

#[derive(Debug, Clone)]
pub struct EnemyBaseStats {
    pub max_health: f32,
    pub move_speed: f32,
    pub size_radius: f32,
}

/// One entry in an enemy's ability list. Same shape as player abilities but simpler:
/// no stance, no talent modification, no hook system (yet).
#[derive(Debug, Clone)]
pub struct EnemyAbilityDef {
    /// Reuses BehaviorRegistry — e.g. "contact_melee", "ranged_projectile".
    pub behavior: BehaviorId,
    pub params: HashMap<StatId, f32>,
    pub cooldown_secs: f32,
}

/// Loaded from assets/themes/<id>.ron.
/// Defines which enemies and bosses can appear in encounters with this theme.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct ThemeDef {
    pub id: ThemeId,
    pub display_name: String,
    /// Pool for normal pack enemies (Map encounters).
    pub common_enemy_pool: Vec<EnemyId>,
    /// Pool for room bosses (BossRoom encounters).
    pub boss_pool: Vec<EnemyId>,
    /// Subset of boss_pool used specifically for Map objective "KillMapBoss".
    pub map_boss_pool: Vec<EnemyId>,
}
