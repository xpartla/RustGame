// AbilityDef — the data template for one ability, loaded from assets/abilities/*.ron.
//
// Each RON file is one ability. The file's `id` field is the stable internal key used
// everywhere: in HeroDef slot maps, TalentDef ability_scope, and AbilityInstance.
//
// NOTE: All names in RON files are WORKING NAMES. The `id` field (snake_case) is the
// stable internal identifier. `display_name` is what players see and can be changed freely.
//
// Interactions:
//   - HeroDef (hero/assets.rs) references AbilityIds in level_1_abilities, band pools,
//     and stance_slots.
//   - TalentDef (talent/assets.rs) references AbilityId in ability_scope.
//   - AbilityInstance (ability/components.rs) stores the AbilityId for runtime lookup.
//   - BehaviorRegistry (ability/behavior.rs) is keyed by the `behavior` field.
//   - HookRegistry (ability/behavior.rs) is keyed by HookIds listed in `hooks`.

use bevy::prelude::*;
use std::collections::HashMap;

// TODO(Phase 1): add `serde` and `ron` to Cargo.toml. Implement AssetLoader for AbilityDef.

/// Internal identifier — stable across renames. Use snake_case. Referenced from HeroDef,
/// TalentDef, and AbilityInstance.
pub type AbilityId = String;

/// Identifies a behavior implementation registered in BehaviorRegistry.
pub type BehaviorId = String;

/// Identifies a hook implementation registered in HookRegistry.
pub type HookId = String;

/// Identifies a numeric parameter within an ability (e.g. "damage", "range", "cooldown").
pub type StatId = String;

/// Loaded from assets/abilities/<id>.ron.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct AbilityDef {
    /// Stable internal key. Must match the filename (without extension).
    pub id: AbilityId,
    /// Player-facing name. Working name — safe to change without affecting any ID lookups.
    pub display_name: String,
    pub unlock_schedule: UnlockSchedule,
    /// Key into BehaviorRegistry — determines how this ability executes.
    pub behavior: BehaviorId,
    /// Ordered list of (phase, hook_id) pairs. The execution system runs pre-hooks before
    /// the base behavior, post-hooks after. Hooks only fire if the player has the
    /// corresponding ActiveHook component (i.e. the talent that installs it is acquired).
    pub hooks: Vec<(HookPhase, HookId)>,
    /// Base numeric parameters consumed by the behavior and modifier stack.
    /// Keys are StatIds; values are the unmodified base values.
    pub base_params: HashMap<StatId, f32>,
    /// Talent IDs that may be offered for this ability. The offer system samples from this
    /// list (plus class_passive_pool and general passives) when generating talent choices.
    pub talent_pool: Vec<String>, // TalentId — String alias, see talent/assets.rs
}

#[derive(Debug, Clone)]
pub enum UnlockSchedule {
    /// Available from level 1 — always granted at run start.
    Level1,
    /// Drawn without replacement from the appropriate band pool during the AbilityUnlock phase.
    Band(u8, u8), // inclusive level range, e.g. Band(2, 3) or Band(4, 6)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookPhase {
    Pre,  // fires before the base behavior
    Post, // fires after the base behavior (receives hit results in ctx)
}
