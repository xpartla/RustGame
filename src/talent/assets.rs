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
        // Heart Strike (Phase 9.2).
        ("heart_strike_extra_target_common", "talents/heart_strike_extra_target_common.talent.ron"),
        ("heart_strike_range_common", "talents/heart_strike_range_common.talent.ron"),
        ("heart_strike_execute_epic", "talents/heart_strike_execute_epic.talent.ron"),
        // Blood Boil (Phase 9.2).
        ("blood_boil_damage_common", "talents/blood_boil_damage_common.talent.ron"),
        ("blood_boil_range_common", "talents/blood_boil_range_common.talent.ron"),
        ("blood_boil_health_scaling_rare", "talents/blood_boil_health_scaling_rare.talent.ron"),
        // BDK class passives (Phase 9.2).
        ("bdk_passive_dnd_damage_boost", "talents/bdk_passive_dnd_damage_boost.talent.ron"),
        ("bdk_passive_blood_boil_spawns_dnd", "talents/bdk_passive_blood_boil_spawns_dnd.talent.ron"),
        ("bdk_passive_no_heal_cap", "talents/bdk_passive_no_heal_cap.talent.ron"),
        ("bdk_passive_overkill_leech", "talents/bdk_passive_overkill_leech.talent.ron"),
        ("bdk_passive_health_and_healing", "talents/bdk_passive_health_and_healing.talent.ron"),
        // Abomination Limb (Phase 9.2).
        ("abomination_limb_range_common", "talents/abomination_limb_range_common.talent.ron"),
        ("abomination_limb_targets_rare", "talents/abomination_limb_targets_rare.talent.ron"),
        ("abomination_limb_stun_rare", "talents/abomination_limb_stun_rare.talent.ron"),
        ("abomination_limb_ranged_only_epic", "talents/abomination_limb_ranged_only_epic.talent.ron"),
        // Purgatory (Phase 9.2).
        ("purgatory_restore_rare", "talents/purgatory_restore_rare.talent.ron"),
        ("purgatory_immunity_epic", "talents/purgatory_immunity_epic.talent.ron"),
        ("purgatory_cooldown_rare", "talents/purgatory_cooldown_rare.talent.ron"),
        // AMZ (Phase 9.2).
        ("amz_size_common", "talents/amz_size_common.talent.ron"),
        ("amz_duration_common", "talents/amz_duration_common.talent.ron"),
        ("amz_regen_rare", "talents/amz_regen_rare.talent.ron"),
        ("amz_movespeed_rare", "talents/amz_movespeed_rare.talent.ron"),
        ("amz_follow_epic", "talents/amz_follow_epic.talent.ron"),
        // Hammer of Justice (Phase 9.3).
        ("hammer_of_justice_damage_common", "talents/hammer_of_justice_damage_common.talent.ron"),
        ("hammer_of_justice_range_common", "talents/hammer_of_justice_range_common.talent.ron"),
        ("hammer_of_justice_shockwave_rare", "talents/hammer_of_justice_shockwave_rare.talent.ron"),
        // Flash of Light (Phase 9.3).
        ("flash_of_light_overheal_shield_common", "talents/flash_of_light_overheal_shield_common.talent.ron"),
        ("flash_of_light_healing_common", "talents/flash_of_light_healing_common.talent.ron"),
        ("flash_of_light_cooldown_common", "talents/flash_of_light_cooldown_common.talent.ron"),
        ("flash_of_light_radiate_rare", "talents/flash_of_light_radiate_rare.talent.ron"),
        ("flash_of_light_consecrated_radiate_epic", "talents/flash_of_light_consecrated_radiate_epic.talent.ron"),
        // Consecrated Ground (Phase 9.3).
        ("consecrated_ground_radius_rare", "talents/consecrated_ground_radius_rare.talent.ron"),
        ("consecrated_ground_damage_common", "talents/consecrated_ground_damage_common.talent.ron"),
        ("consecrated_ground_slow_common", "talents/consecrated_ground_slow_common.talent.ron"),
        ("consecrated_ground_count_scaling_rare", "talents/consecrated_ground_count_scaling_rare.talent.ron"),
        // Spinning Hammer (Phase 9.3).
        ("spinning_hammer_damage_common", "talents/spinning_hammer_damage_common.talent.ron"),
        ("spinning_hammer_radius_common", "talents/spinning_hammer_radius_common.talent.ron"),
        ("spinning_hammer_stun_rare", "talents/spinning_hammer_stun_rare.talent.ron"),
        ("spinning_hammer_extra_hammer_epic", "talents/spinning_hammer_extra_hammer_epic.talent.ron"),
        // Smite (Phase 9.3).
        ("smite_damage_common", "talents/smite_damage_common.talent.ron"),
        ("smite_range_common", "talents/smite_range_common.talent.ron"),
        ("smite_extra_target_rare", "talents/smite_extra_target_rare.talent.ron"),
        ("smite_spawns_consecrated_rare", "talents/smite_spawns_consecrated_rare.talent.ron"),
        ("smite_mark_radius_epic", "talents/smite_mark_radius_epic.talent.ron"),
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

    #[test]
    fn amz_regen_rare_overrides_regen_percent_per_second() {
        let def = load("assets/talents/amz_regen_rare.talent.ron");
        match def.effect {
            TalentEffect::Modifier(StatModifier { ref stat, op: ModOp::Override(v) }) => {
                assert_eq!(stat, "regen_percent_per_second");
                assert_eq!(v, 0.5);
            }
            _ => panic!("expected an Override modifier"),
        }
    }

    #[test]
    fn amz_follow_epic_overrides_follow_caster() {
        let def = load("assets/talents/amz_follow_epic.talent.ron");
        assert_eq!(def.rarity, TalentRarity::Epic);
        match def.effect {
            TalentEffect::Modifier(StatModifier { ref stat, op: ModOp::Override(v) }) => {
                assert_eq!(stat, "follow_caster");
                assert_eq!(v, 1.0);
            }
            _ => panic!("expected an Override modifier"),
        }
    }

    #[test]
    fn amz_movespeed_rare_is_a_behavior_flag() {
        let def = load("assets/talents/amz_movespeed_rare.talent.ron");
        match def.effect {
            TalentEffect::Behavior(ref hook) => assert_eq!(hook, "amz_movespeed"),
            _ => panic!("expected a Behavior effect"),
        }
    }

    #[test]
    fn bdk_class_passives_parse_with_expected_effects() {
        let no_heal_cap = load("assets/talents/bdk_passive_no_heal_cap.talent.ron");
        assert_eq!(no_heal_cap.ability_scope, None, "class-wide passive");
        assert!(matches!(no_heal_cap.effect, TalentEffect::Behavior(ref h) if h == "bdk_no_heal_cap"));

        let overkill = load("assets/talents/bdk_passive_overkill_leech.talent.ron");
        assert_eq!(overkill.rarity, TalentRarity::Rare);
        assert!(matches!(overkill.effect, TalentEffect::Behavior(ref h) if h == "bdk_overkill_leech"));

        let health_healing = load("assets/talents/bdk_passive_health_and_healing.talent.ron");
        assert_eq!(health_healing.rarity, TalentRarity::Common);
        assert!(matches!(health_healing.uniqueness, UniquenessConstraint::Stack(3)));
        assert!(matches!(health_healing.effect, TalentEffect::Behavior(ref h) if h == "bdk_health_and_healing"));

        let spawns_dnd = load("assets/talents/bdk_passive_blood_boil_spawns_dnd.talent.ron");
        assert!(matches!(spawns_dnd.effect, TalentEffect::Behavior(ref h) if h == "bdk_blood_boil_spawns_dnd"));
    }

    /// Every Paladin talent (Phase 9.3) parses through the same RON path the AssetLoader uses.
    /// One broad test rather than 21 near-identical ones — the interesting assertions (scope,
    /// rarity, effect shape) are covered per-ability by the more targeted tests below.
    #[test]
    fn all_paladin_talents_parse() {
        let ids = [
            "hammer_of_justice_damage_common",
            "hammer_of_justice_range_common",
            "hammer_of_justice_shockwave_rare",
            "flash_of_light_overheal_shield_common",
            "flash_of_light_healing_common",
            "flash_of_light_cooldown_common",
            "flash_of_light_radiate_rare",
            "flash_of_light_consecrated_radiate_epic",
            "consecrated_ground_radius_rare",
            "consecrated_ground_damage_common",
            "consecrated_ground_slow_common",
            "consecrated_ground_count_scaling_rare",
            "spinning_hammer_damage_common",
            "spinning_hammer_radius_common",
            "spinning_hammer_stun_rare",
            "spinning_hammer_extra_hammer_epic",
            "smite_damage_common",
            "smite_range_common",
            "smite_extra_target_rare",
            "smite_spawns_consecrated_rare",
            "smite_mark_radius_epic",
        ];
        for id in ids {
            let def = load(&format!("assets/talents/{id}.talent.ron"));
            assert_eq!(def.id, id);
        }
    }

    #[test]
    fn spinning_hammer_stun_and_extra_hammer_are_shaped_as_designed() {
        let stun = load("assets/talents/spinning_hammer_stun_rare.talent.ron");
        assert_eq!(stun.ability_scope.as_deref(), Some("spinning_hammer"));
        assert!(matches!(stun.effect, TalentEffect::Behavior(ref h) if h == "spinning_hammer_stun"));

        let extra = load("assets/talents/spinning_hammer_extra_hammer_epic.talent.ron");
        match extra.effect {
            TalentEffect::Modifier(StatModifier { ref stat, op: ModOp::Add(v) }) => {
                assert_eq!(stat, "hammer_count");
                assert_eq!(v, 1.0);
            }
            _ => panic!("expected an Add modifier on hammer_count"),
        }
    }

    #[test]
    fn consecrated_ground_slow_and_count_scaling_override_their_flags() {
        let slow = load("assets/talents/consecrated_ground_slow_common.talent.ron");
        match slow.effect {
            TalentEffect::Modifier(StatModifier { ref stat, op: ModOp::Override(v) }) => {
                assert_eq!(stat, "slow_active");
                assert_eq!(v, 1.0);
            }
            _ => panic!("expected an Override modifier on slow_active"),
        }

        let scaling = load("assets/talents/consecrated_ground_count_scaling_rare.talent.ron");
        match scaling.effect {
            TalentEffect::Modifier(StatModifier { ref stat, op: ModOp::Override(v) }) => {
                assert_eq!(stat, "count_scaling_active");
                assert_eq!(v, 1.0);
            }
            _ => panic!("expected an Override modifier on count_scaling_active"),
        }
    }

    #[test]
    fn smite_spawns_consecrated_and_mark_radius_are_behavior_flags() {
        let spawn = load("assets/talents/smite_spawns_consecrated_rare.talent.ron");
        assert!(matches!(spawn.effect, TalentEffect::Behavior(ref h) if h == "smite_spawns_consecrated"));

        let mark = load("assets/talents/smite_mark_radius_epic.talent.ron");
        assert!(matches!(mark.effect, TalentEffect::Behavior(ref h) if h == "smite_mark_radius"));
    }
}
