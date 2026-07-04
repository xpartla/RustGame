// TalentDef — one file per talent, loaded from assets/talents/*.ron.
//
// NOTE: All names are WORKING NAMES. The `id` field is the stable internal key.
// `display_name` is what players see and can be changed freely.
//
// Naming convention for talent IDs: <ability_id>_<description>_<rarity>
// Example: "death_strike_leech_common", "death_strike_bone_shield_epic"
// Class-wide passives: "bdk_passive_<description>_<rarity>"
// General passives: "general_<description>_<rarity>"
//
// Interactions:
//   - AbilityDef.talent_pool lists TalentIds offered for that ability.
//   - HeroDef.class_passive_pool lists class-wide TalentIds.
//   - AcquiredTalents (talent/components.rs) stores the player's acquired list.
//   - progression/systems/offer.rs samples from all eligible pools to generate offers.
//   - talent/systems/apply.rs installs ActiveHook when a Behavior talent is acquired.

use bevy::prelude::*;
use crate::ability::assets::{AbilityId, HookId, StatId};

pub type TalentId = String;

/// Loaded from assets/talents/<id>.ron.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct TalentDef {
    /// Stable internal key. Must match the filename (without extension).
    pub id: TalentId,
    /// Player-facing name. Working name — safe to change.
    pub display_name: String,
    /// None = class-wide or general passive (applies globally).
    /// Some(ability_id) = scoped to one ability's modifier stack.
    pub ability_scope: Option<AbilityId>,
    pub rarity: TalentRarity,
    pub uniqueness: UniquenessConstraint,
    pub effect: TalentEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TalentRarity {
    Common,
    Rare,
    Epic,
}

#[derive(Debug, Clone)]
pub enum UniquenessConstraint {
    /// No limit — can be offered and taken as many times as the pool allows.
    None,
    /// unique[N] — can be taken at most N times total.
    Stack(u8),
    /// Only one copy — once acquired, never offered again.
    Exclusive,
    /// Taking this forecloses the named talent (and vice versa — both sides declare it).
    /// Example: Fiery Ent and Earth Ent each declare MutuallyExcludes on the other.
    MutuallyExcludes(TalentId),
}

#[derive(Debug, Clone)]
pub enum TalentEffect {
    /// Pure data — handled by the modifier stack in resolve_params(). No code hook needed.
    Modifier(StatModifier),
    /// Behavior-rewriting — installs an ActiveHook component on the player when acquired.
    /// The hook executes only when that component is present. Removing the talent removes
    /// the component (merchant remove-talent path).
    Behavior(HookId),
}

#[derive(Debug, Clone)]
pub struct StatModifier {
    /// The stat key this modifier applies to (must match a key in AbilityDef.base_params).
    pub stat: StatId,
    pub op: ModOp,
}

#[derive(Debug, Clone)]
pub enum ModOp {
    /// Adds a flat value: new = base + sum(Add)
    Add(f32),
    /// Multiplicative bonus stacked additively: new = base * (1 + sum(MultiplyAdd))
    MultiplyAdd(f32),
    /// Replaces the stat entirely. Use sparingly; for epic-level behavior changes.
    Override(f32),
}
