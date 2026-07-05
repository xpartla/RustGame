// AbilityDef — the data template for one ability, loaded from assets/abilities/*.ability.ron.
//
// Each RON file is one ability. The file's `id` field is the stable internal key used
// everywhere: in HeroDef slot maps, TalentDef ability_scope, and AbilityInstance.
//
// NOTE: All names in RON files are WORKING NAMES. The `id` field (snake_case) is the
// stable internal identifier. `display_name` is what players see and can be changed freely.
//
// File extension is `.ability.ron` so the loader can be registered unambiguously alongside
// the other RON asset types added in later phases (talents, heroes, enemies, themes).
//
// Interactions:
//   - HeroDef (hero/assets.rs) references AbilityIds in level_1_abilities, band pools, slots.
//   - TalentDef (talent/assets.rs) references AbilityId in ability_scope.
//   - AbilityInstance (ability/components.rs) stores the AbilityId for runtime lookup.
//   - BehaviorRegistry (ability/behavior.rs) is keyed by the `behavior` field.
//   - HookRegistry (arrives with the talent system in Phase 2) will be keyed by the HookIds
//     listed in `hooks`.
//   - AbilityLibrary (below) maps AbilityId → Handle<AbilityDef> so runtime systems can
//     resolve an AbilityInstance's string id to the loaded asset.

use bevy::prelude::*;
use std::collections::HashMap;
use crate::core::def_library::{DefAsset, DefLibrary};
use crate::core::events::DamageTag;

/// Internal identifier — stable across renames. Use snake_case. Referenced from HeroDef,
/// TalentDef, and AbilityInstance.
pub type AbilityId = String;

/// Identifies a behavior implementation registered in BehaviorRegistry.
pub type BehaviorId = String;

/// Identifies a hook implementation, resolved via the HookRegistry that arrives with the
/// talent system in Phase 2.
pub type HookId = String;

/// Identifies a numeric parameter within an ability (e.g. "damage", "range", "cooldown").
pub type StatId = String;

/// One declarative outcome an ability applies to the entities its behavior resolved as hit.
/// The behavior decides *which* entities (the CastOutcome); the effect list decides *what happens*.
/// Numeric fields reference param keys (StatId) so the talent modifier stack reaches every number.
///
/// `ApplyStatus` arrives with the status module (Phase 3B); Phase 3A ships Damage/Heal/Leech.
#[derive(Debug, Clone, serde::Deserialize)]
pub enum EffectSpec {
    /// Deal `amount` (a param key) to the selected targets, tagged with `tags`.
    Damage { amount: StatId, tags: Vec<DamageTag>, target: EffectTarget },
    /// Restore `amount` (a param key) to the selected targets.
    Heal { amount: StatId, target: EffectTarget },
    /// Heal the caster for `percent` (a param key) of the total damage this cast dealt.
    Leech { percent: StatId },
    /// Apply `stacks` of a status effect (by id) to the selected targets. Emits ApplyStatusEvent;
    /// stacking / duration / DoT are the StatusEffectDef's concern (status/assets.rs).
    ApplyStatus { status: String, stacks: u8, target: EffectTarget },
}

/// Which entities from the behavior's CastOutcome an EffectSpec applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum EffectTarget {
    /// Every entity the behavior hit.
    AllHits,
    /// The nearest/first hit only (single-target abilities, projectile impact).
    PrimaryHit,
    /// The caster (self-heal, self-buff).
    Caster,
}

/// Loaded from assets/abilities/<id>.ability.ron.
///
/// Several fields are deserialized from RON but not yet read by game logic — they are consumed
/// in later phases (`unlock_schedule` by progression, `display_name` by UI, `talent_pool` by the
/// offer generator, `hooks` by the talent hook system). `#[allow(dead_code)]` until then.
#[allow(dead_code)]
#[derive(Asset, TypePath, Debug, Clone, serde::Deserialize)]
pub struct AbilityDef {
    /// Stable internal key. Must match the filename stem (without extension).
    pub id: AbilityId,
    /// Player-facing name. Working name — safe to change without affecting any ID lookups.
    pub display_name: String,
    pub unlock_schedule: UnlockSchedule,
    /// How the ability fires: Input (default) or AutoCast (on cooldown, no input).
    #[serde(default)]
    pub activation: Activation,
    /// Key into BehaviorRegistry — determines how this ability executes.
    pub behavior: BehaviorId,
    /// Ordered list of (phase, hook_id) pairs. The execution system runs pre-hooks before
    /// the base behavior, post-hooks after. Hooks only fire if the player has the
    /// corresponding ActiveHook component (i.e. the talent that installs it is acquired).
    /// Not yet consumed — hook execution lands with the talent system (Phase 2).
    pub hooks: Vec<(HookPhase, HookId)>,
    /// Base numeric parameters consumed by the behavior and modifier stack.
    /// Keys are StatIds; values are the unmodified base values.
    pub base_params: HashMap<StatId, f32>,
    /// Talent IDs that may be offered for this ability. The offer system samples from this
    /// list (plus class_passive_pool and general passives) when generating talent choices.
    pub talent_pool: Vec<String>, // TalentId — String alias, see talent/assets.rs
    /// Declarative gameplay outcomes applied to the entities the behavior resolves as hit.
    /// `#[serde(default)]` so an un-migrated / behavior-only ability parses with no effects
    /// (inert). Applied by ability/systems/execute.rs::apply_effects.
    #[serde(default)]
    pub effects: Vec<EffectSpec>,
}

/// How an ability is triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
pub enum Activation {
    /// Fired by an input slot (LMB/RMB) via TriggerAbilityEvent. The default.
    #[default]
    Input,
    /// Fired automatically whenever its cooldown is ready (passive abilities: Blood Boil,
    /// Flamewrath, Consecrated Ground, …). Driven by `auto_cast_abilities`.
    AutoCast,
}

// Read by the progression/unlock flow (Phase 2); deserialized-but-unread until then.
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
pub enum UnlockSchedule {
    /// Available from level 1 — always granted at run start.
    Level1,
    /// Drawn without replacement from the appropriate band pool during the AbilityUnlock phase.
    Band(u8, u8), // inclusive level range, e.g. Band(2, 3) or Band(4, 6)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub enum HookPhase {
    Pre,  // fires before the base behavior
    Post, // fires after the base behavior (receives hit results in ctx)
}

/// Resource mapping AbilityId → Handle<AbilityDef>. A `DefLibrary<AbilityDef>` (see
/// core/def_library.rs); populated at startup from `AbilityDef::MANIFEST` via
/// `register_def_library::<AbilityDef>()`, read by the execution system to resolve an
/// `AbilityInstance.def_id` string to the actual `AbilityDef`.
pub type AbilityLibrary = DefLibrary<AbilityDef>;

impl DefAsset for AbilityDef {
    const EXTENSIONS: &'static [&'static str] = &["ability.ron"];
    const MANIFEST: &'static [(&'static str, &'static str)] = &[
        ("death_strike", "abilities/death_strike.ability.ron"),
        ("dnd", "abilities/dnd.ability.ron"),
        // Phase 3 demonstrators. Fireblast/Frostbolt are bound to the Mage's Fire/Ice stances
        // in Phase 4; Scratch stays an unbound demonstrator until the Druid (later phase).
        ("fireblast", "abilities/fireblast.ability.ron"),
        ("frostbolt", "abilities/frostbolt.ability.ron"),
        ("scratch", "abilities/scratch.ability.ron"),
        // Blood Boil: BDK L2/3 band ability, live as an auto-cast self-nova (Phase 3).
        ("blood_boil", "abilities/blood_boil.ability.ron"),
    ];
}

#[cfg(test)]
mod tests {
    //! Parse the real RON asset files through the same `ron::de` path the AssetLoader uses.
    //! Headless — runs under `cargo test` without a window/GPU.
    use super::*;

    fn load(rel_path: &str) -> AbilityDef {
        let full = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel_path);
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("read {full}: {e}"));
        ron::de::from_bytes::<AbilityDef>(&bytes)
            .unwrap_or_else(|e| panic!("parse {rel_path}: {e}"))
    }

    #[test]
    fn death_strike_parses() {
        let def = load("assets/abilities/death_strike.ability.ron");
        assert_eq!(def.id, "death_strike");
        assert_eq!(def.behavior, "melee_cone");
        assert!(matches!(def.unlock_schedule, UnlockSchedule::Level1));
        assert_eq!(def.base_params.get("damage"), Some(&10.0));
        assert_eq!(def.base_params.get("range"), Some(&60.0));
        assert_eq!(def.base_params.get("leech_percent"), Some(&5.0));
        assert_eq!(def.hooks.len(), 1);
        assert_eq!(def.hooks[0], (HookPhase::Post, "bone_shield_on_kill".to_string()));
        assert!(def.talent_pool.contains(&"death_strike_leech_common".to_string()));
        // Phase 3 generic-effect list: physical damage to all hits + leech.
        assert_eq!(def.effects.len(), 2);
        assert!(matches!(
            def.effects[0],
            EffectSpec::Damage { target: EffectTarget::AllHits, .. }
        ));
        assert!(matches!(def.effects[1], EffectSpec::Leech { .. }));
    }

    #[test]
    fn ability_without_effects_defaults_to_empty() {
        // dnd.ability.ron declares no `effects` — serde(default) yields an inert (empty) list.
        let def = load("assets/abilities/dnd.ability.ron");
        assert!(def.effects.is_empty());
    }

    #[test]
    fn dnd_parses() {
        let def = load("assets/abilities/dnd.ability.ron");
        assert_eq!(def.id, "dnd");
        assert_eq!(def.behavior, "dropped_zone");
        assert!(def.hooks.is_empty());
        assert_eq!(def.base_params.get("zone_radius"), Some(&80.0));
    }
}
