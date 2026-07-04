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

use bevy::asset::{io::Reader, AssetLoader, LoadContext};
use bevy::prelude::*;
use std::collections::HashMap;

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

/// Asset loader for `*.ability.ron`. Registered in `AbilityPlugin::build`.
#[derive(Default)]
pub struct AbilityDefLoader;

impl AssetLoader for AbilityDefLoader {
    type Asset = AbilityDef;
    type Settings = ();
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<AbilityDef, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let def = ron::de::from_bytes::<AbilityDef>(&bytes)?;
        Ok(def)
    }

    fn extensions(&self) -> &[&str] {
        &["ability.ron"]
    }
}

/// Resource: maps an AbilityId to the handle of its loaded AbilityDef asset.
/// Populated at startup (`load_ability_defs`); read by the execution system to resolve an
/// `AbilityInstance.def_id` string to the actual `AbilityDef`.
#[derive(Resource, Default)]
pub struct AbilityLibrary {
    pub defs: HashMap<AbilityId, Handle<AbilityDef>>,
}

impl AbilityLibrary {
    pub fn get(&self, id: &str) -> Option<&Handle<AbilityDef>> {
        self.defs.get(id)
    }
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
