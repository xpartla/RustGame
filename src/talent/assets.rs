// TalentDef — one file per talent, loaded from assets/talents/*.talent.ron.
//
// NOTE: All names are WORKING NAMES. The `id` field is the stable internal key.
// `display_name` is what players see and can be changed freely.
//
// Naming convention for talent IDs: <ability_id>_<description>_<rarity>
// Example: "death_strike_leech_common", "death_strike_bone_shield_epic"
// Class-wide passives: "bdk_passive_<description>_<rarity>"
// General passives: "general_<description>_<rarity>"
//
// File extension is `.talent.ron` (mirroring `.ability.ron`) so the loader registers
// unambiguously alongside the ability loader and future hero/enemy/theme loaders — no
// collisions on a shared plain `.ron`.
//
// Interactions:
//   - AbilityDef.talent_pool lists TalentIds offered for that ability.
//   - HeroDef.class_passive_pool lists class-wide TalentIds.
//   - AcquiredTalents (talent/components.rs) stores the player's acquired list.
//   - progression/systems/offer.rs samples from all eligible pools to generate offers.
//   - talent/systems/apply.rs installs ActiveHook when a Behavior talent is acquired.
//   - talent/modifier.rs::resolve_params reads Modifier talents to build the stat stack.
//   - TalentLibrary (below) maps TalentId → Handle<TalentDef> so runtime systems can
//     resolve an acquired talent's string id to the loaded asset (mirrors AbilityLibrary).

use bevy::prelude::*;
use crate::ability::assets::{AbilityId, HookId, StatId};
use crate::core::def_library::{DefAsset, DefLibrary};

pub type TalentId = String;

/// Loaded from assets/talents/<id>.talent.ron.
#[derive(Asset, TypePath, Debug, Clone, serde::Deserialize)]
pub struct TalentDef {
    /// Stable internal key. Must match the filename stem (without extension).
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum TalentRarity {
    Common,
    Rare,
    Epic,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TalentEffect {
    /// Pure data — handled by the modifier stack in resolve_params(). No code hook needed.
    Modifier(StatModifier),
    /// Behavior-rewriting — installs an ActiveHook component on the player when acquired.
    /// The hook executes only when that component is present. Removing the talent removes
    /// the component (merchant remove-talent path).
    Behavior(HookId),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatModifier {
    /// The stat key this modifier applies to (must match a key in AbilityDef.base_params).
    pub stat: StatId,
    pub op: ModOp,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ModOp {
    /// Adds a flat value: new = base + sum(Add)
    Add(f32),
    /// Multiplicative bonus stacked additively: new = base * (1 + sum(MultiplyAdd))
    MultiplyAdd(f32),
    /// Replaces the stat entirely. Use sparingly; for epic-level behavior changes.
    Override(f32),
}

/// Resource mapping TalentId → Handle<TalentDef>. A `DefLibrary<TalentDef>` (see
/// core/def_library.rs); populated at startup from `TalentDef::MANIFEST`, read by resolve_params
/// (modifier stack) and the offer generator to resolve an acquired/eligible talent id to the
/// actual `TalentDef`.
pub type TalentLibrary = DefLibrary<TalentDef>;

impl DefAsset for TalentDef {
    const EXTENSIONS: &'static [&'static str] = &["talent.ron"];
    const MANIFEST: &'static [(&'static str, &'static str)] = &[
        ("death_strike_leech_common", "talents/death_strike_leech_common.talent.ron"),
        ("death_strike_range_common", "talents/death_strike_range_common.talent.ron"),
        ("death_strike_damage_common", "talents/death_strike_damage_common.talent.ron"),
        ("death_strike_bone_shield_epic", "talents/death_strike_bone_shield_epic.talent.ron"),
        ("blood_boil_dnd_range_rare", "talents/blood_boil_dnd_range_rare.talent.ron"),
    ];
}

#[cfg(test)]
mod tests {
    //! Parse the real RON asset files through the same `ron::de` path the AssetLoader uses.
    //! Headless — runs under `cargo test` without a window/GPU.
    use super::*;

    fn load(rel_path: &str) -> TalentDef {
        let full = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel_path);
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("read {full}: {e}"));
        ron::de::from_bytes::<TalentDef>(&bytes)
            .unwrap_or_else(|e| panic!("parse {rel_path}: {e}"))
    }

    #[test]
    fn leech_common_parses() {
        let def = load("assets/talents/death_strike_leech_common.talent.ron");
        assert_eq!(def.id, "death_strike_leech_common");
        assert_eq!(def.ability_scope.as_deref(), Some("death_strike"));
        assert_eq!(def.rarity, TalentRarity::Common);
        assert!(matches!(def.uniqueness, UniquenessConstraint::Stack(3)));
        match def.effect {
            TalentEffect::Modifier(StatModifier { ref stat, op: ModOp::MultiplyAdd(v) }) => {
                assert_eq!(stat, "leech_percent");
                assert!((v - 0.20).abs() < 1e-6);
            }
            _ => panic!("expected MultiplyAdd modifier on leech_percent"),
        }
    }

    #[test]
    fn range_and_damage_common_parse() {
        let range = load("assets/talents/death_strike_range_common.talent.ron");
        assert_eq!(range.id, "death_strike_range_common");
        let dmg = load("assets/talents/death_strike_damage_common.talent.ron");
        assert_eq!(dmg.id, "death_strike_damage_common");
        assert!(matches!(dmg.effect, TalentEffect::Modifier(_)));
    }

    #[test]
    fn bone_shield_epic_is_behavior() {
        let def = load("assets/talents/death_strike_bone_shield_epic.talent.ron");
        assert_eq!(def.rarity, TalentRarity::Epic);
        assert!(matches!(def.uniqueness, UniquenessConstraint::Exclusive));
        match def.effect {
            TalentEffect::Behavior(ref hook) => assert_eq!(hook, "bone_shield_on_kill"),
            _ => panic!("expected Behavior effect"),
        }
    }

    #[test]
    fn blood_boil_dnd_range_rare_parses() {
        let def = load("assets/talents/blood_boil_dnd_range_rare.talent.ron");
        assert_eq!(def.rarity, TalentRarity::Rare);
        assert_eq!(def.ability_scope.as_deref(), Some("blood_boil"));
    }
}
