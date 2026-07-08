// HeroDef — one file per class, loaded from assets/heroes/<id>.ron.
//
// NOTE: All class names, ability names referenced here are WORKING NAMES. The `id` field
// is the stable internal key. `display_name` is what players see on the character select screen.
//
// Stance system:
//   - Classes with has_stance == false (Death Knight, Paladin) have no Q binding.
//     The stance_slots list contains a single entry with stance == "default".
//   - Classes with has_stance == true (Druid, Mage) have two entries (one per form).
//     Q triggers handle_stance_swap which swaps ActiveStance and fires the stance-swap ability.
//
// Interactions:
//   - hero/systems/input_slot.rs reads stance_slots to resolve InputSlot → AbilityId.
//   - hero/systems/stance.rs reads stance_a / stance_b for the swap animation cue.
//   - progression/systems/level_up.rs reads band_2_3_pool / band_4_6_pool.
//   - progression/systems/offer.rs reads class_passive_pool for talent offer generation.

use bevy::prelude::*;
use crate::ability::assets::AbilityId;
use crate::core::def_library::{DefAsset, DefLibrary};
use crate::status::assets::StatusEffectId;
use crate::talent::assets::TalentId;

pub type HeroId = String;
pub type StanceId = String;

/// Loaded from assets/heroes/<id>.hero.ron.
#[derive(Asset, TypePath, Debug, Clone, serde::Deserialize)]
pub struct HeroDef {
    pub id: HeroId,
    pub display_name: String,
    pub base_stats: HeroBaseStats,
    pub resource_model: ResourceModel,
    /// true for Druid and Mage (Q swaps stance); false for Death Knight and Paladin.
    pub has_stance: bool,
    /// Only meaningful when has_stance == true.
    pub stance_a: Option<StanceId>,
    pub stance_b: Option<StanceId>,
    /// Always unlocked at level 1, regardless of band pools.
    pub level_1_abilities: Vec<AbilityId>,
    /// Draw one at each of levels 2 and 3 (without replacement).
    pub band_2_3_pool: Vec<AbilityId>,
    /// Draw one at each of levels 4, 5, and 6 (without replacement, from this pool).
    pub band_4_6_pool: Vec<AbilityId>,
    /// Class-specific passive talents available in the offer pool once TalentChoices phase begins.
    pub class_passive_pool: Vec<TalentId>,
    /// Maps each stance to its InputSlot bindings.
    /// Non-stance heroes have a single entry: stance == "default".
    pub stance_slots: Vec<StanceSlotMapping>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct HeroBaseStats {
    pub max_health: f32,
    pub move_speed: f32,
}

/// How the class interacts with resources (mana, health, charges).
#[derive(Debug, Clone, serde::Deserialize)]
pub enum ResourceModel {
    /// No secondary resource bar. Standard health-only class (Druid, Mage, Paladin prototype).
    None,
    /// Health IS the gameplay resource — some abilities cost health or scale with missing health.
    /// No secondary bar; the existing Health component is the resource.
    HealthBased,
    /// A capped integer charge bar (Phase 9.1, §3.11 of the phase-9 plan) — Mage frost charges and
    /// Druid enhanced/combo charges are the first consumers (Phase 9.4/9.5). `max` is content; the
    /// runtime count lives in `hero::components::Charges`. Transient (not part of `RunState` — a
    /// charge count is mid-encounter state, reset like the rest of live combat state on resume).
    Charges { max: u32 },
}

/// Binds InputSlots to AbilityIds for one stance.
/// Slots without an ability (e.g. StanceSwap for non-stance heroes) are absent from the map.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StanceSlotMapping {
    pub stance: StanceId,
    pub basic: Option<AbilityId>,
    pub special: Option<AbilityId>,
    pub movement: Option<AbilityId>,
    // StanceSwap is handled separately by hero/systems/stance.rs, not as a normal ability.
    /// Status effect applied to the caster when *entering* this stance (Phase 4). For the Mage:
    /// entering Fire grants "boots_of_fire" (move-speed buff); entering Ice grants "ice_barrier"
    /// (damage-reduction). `None` for stances with no on-swap effect (e.g. the "default" stance).
    #[serde(default)]
    pub swap_effect: Option<StatusEffectId>,
    /// Whether entering this stance also fires its own `basic` ability (Phase 9.4 — the Druid:
    /// "change from human to animal form and cast Scratch," "change from animal to human and cast
    /// Roots" — literally the stance's own Basic slot, not a separate cast). `false` for every
    /// stance that only applies a `swap_effect` (the Mage). `#[serde(default)]` so every pre-9.4
    /// hero RON parses unchanged.
    #[serde(default)]
    pub cast_on_enter: bool,
}

/// Resource mapping HeroId → Handle<HeroDef>. A `DefLibrary<HeroDef>` (see core/def_library.rs);
/// populated at startup from `HeroDef::MANIFEST` via `register_def_library::<HeroDef>()`. Read by
/// the hero input/stance systems and the deferred level-1 ability grant.
pub type HeroLibrary = DefLibrary<HeroDef>;

impl DefAsset for HeroDef {
    // Compound `.hero.ron` extension so the loader never collides with plain `.ron` (mirrors
    // `.ability.ron` / `.talent.ron` / `.status.ron`).
    const EXTENSIONS: &'static [&'static str] = &["hero.ron"];
    const MANIFEST: &'static [(&'static str, &'static str)] = &[
        ("blood_death_knight", "heroes/blood_death_knight.hero.ron"),
        ("mage", "heroes/mage.hero.ron"),
        ("paladin", "heroes/paladin.hero.ron"),
        ("druid", "heroes/druid.hero.ron"),
    ];
}

#[cfg(test)]
mod tests {
    //! Parse the real HeroDef RON files through the same `ron::de` path the AssetLoader uses.
    //! Headless — runs under `cargo test` without a window/GPU.
    use super::*;

    fn load(rel_path: &str) -> HeroDef {
        let full = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel_path);
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("read {full}: {e}"));
        ron::de::from_bytes::<HeroDef>(&bytes)
            .unwrap_or_else(|e| panic!("parse {rel_path}: {e}"))
    }

    #[test]
    fn blood_death_knight_parses_as_non_stance_hero() {
        let def = load("assets/heroes/blood_death_knight.hero.ron");
        assert_eq!(def.id, "blood_death_knight");
        assert!(!def.has_stance, "Death Knight has no Q stance swap");
        assert!(def.stance_a.is_none());
        assert!(def.stance_b.is_none());
        assert!(matches!(def.resource_model, ResourceModel::HealthBased));
        assert_eq!(def.base_stats.max_health, 200.0);
        // Level-1 grant list — must match ability/plugin.rs's previous hardcoded stub exactly.
        assert_eq!(def.level_1_abilities, vec!["death_strike", "dnd", "companion"]);
        // Single "default" stance mapping: LMB→death_strike, RMB→dnd, no swap effect.
        assert_eq!(def.stance_slots.len(), 1);
        let slot = &def.stance_slots[0];
        assert_eq!(slot.stance, "default");
        assert_eq!(slot.basic.as_deref(), Some("death_strike"));
        assert_eq!(slot.special.as_deref(), Some("dnd"));
        assert!(slot.movement.is_none());
        assert!(slot.swap_effect.is_none());
    }

    #[test]
    fn mage_parses_as_two_stance_hero() {
        let def = load("assets/heroes/mage.hero.ron");
        assert_eq!(def.id, "mage");
        assert!(def.has_stance, "Mage swaps Fire/Ice with Q");
        assert_eq!(def.stance_a.as_deref(), Some("fire"));
        assert_eq!(def.stance_b.as_deref(), Some("ice"));
        assert!(matches!(def.resource_model, ResourceModel::None));
        // Both basics are level-1 so either stance's LMB works immediately.
        assert_eq!(def.level_1_abilities, vec!["fireblast", "frostbolt"]);
        assert_eq!(def.stance_slots.len(), 2);

        let fire = def.stance_slots.iter().find(|m| m.stance == "fire").expect("fire stance");
        assert_eq!(fire.basic.as_deref(), Some("fireblast"));
        assert_eq!(fire.swap_effect.as_deref(), Some("boots_of_fire"));

        let ice = def.stance_slots.iter().find(|m| m.stance == "ice").expect("ice stance");
        assert_eq!(ice.basic.as_deref(), Some("frostbolt"));
        assert_eq!(ice.swap_effect.as_deref(), Some("ice_barrier"));
    }

    #[test]
    fn paladin_parses_as_non_stance_hero_with_a_single_band() {
        let def = load("assets/heroes/paladin.hero.ron");
        assert_eq!(def.id, "paladin");
        assert!(!def.has_stance, "Paladin has no Q stance swap (architecture-plan §6 Q4)");
        assert!(def.stance_a.is_none());
        assert!(def.stance_b.is_none());
        assert!(matches!(def.resource_model, ResourceModel::None));
        assert_eq!(def.level_1_abilities, vec!["hammer_of_justice", "flash_of_light"]);
        // All three band abilities unlock at levels 2/3/4 (Mechanics) — a single pool, not split
        // across band_2_3/band_4_6 like the BDK.
        assert_eq!(def.band_2_3_pool, vec!["consecrated_ground", "spinning_hammer", "smite"]);
        assert!(def.band_4_6_pool.is_empty());
        let slot = &def.stance_slots[0];
        assert_eq!(slot.stance, "default");
        assert_eq!(slot.basic.as_deref(), Some("hammer_of_justice"));
        assert_eq!(slot.special.as_deref(), Some("flash_of_light"));
    }

    #[test]
    fn druid_parses_as_a_two_stance_charges_hero_that_casts_on_stance_entry() {
        let def = load("assets/heroes/druid.hero.ron");
        assert_eq!(def.id, "druid");
        assert!(def.has_stance, "Druid swaps Human/Animal with Q");
        assert_eq!(def.stance_a.as_deref(), Some("human"));
        assert_eq!(def.stance_b.as_deref(), Some("animal"));
        assert!(matches!(def.resource_model, ResourceModel::Charges { max: 3 }));
        // All four Basic/Special abilities across both stances are owned from level 1.
        assert_eq!(def.level_1_abilities, vec!["scratch", "ferocious_bite", "roots", "heal"]);
        assert_eq!(def.band_2_3_pool, vec!["primal_pounce", "spawn_ent"]);
        assert_eq!(def.band_4_6_pool, vec!["tree_conduit", "bloom"]);
        assert_eq!(def.stance_slots.len(), 2);

        let human = def.stance_slots.iter().find(|m| m.stance == "human").expect("human stance");
        assert_eq!(human.basic.as_deref(), Some("roots"));
        assert_eq!(human.special.as_deref(), Some("heal"));
        assert!(human.cast_on_enter, "entering Human casts Roots (its own Basic)");
        assert!(human.swap_effect.is_none(), "no buff status — cast_on_enter replaces that model");

        let animal = def.stance_slots.iter().find(|m| m.stance == "animal").expect("animal stance");
        assert_eq!(animal.basic.as_deref(), Some("scratch"));
        assert_eq!(animal.special.as_deref(), Some("ferocious_bite"));
        assert!(animal.cast_on_enter, "entering Animal casts Scratch (its own Basic)");
    }
}
