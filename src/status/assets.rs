// StatusEffectDef — one file per effect, loaded from assets/status_effects/*.status.ron.
//
// Phase 3 makes the six built-in effects fully DECLARATIVE — zero Rust per effect. A status is
// described by data:
//   tick               — optional damage-over-time (interval + flat damage + element tags)
//   move_speed_mult    — multiplies the target's velocity while active (frostbite 0.8)
//   damage_taken_mult  — multiplies incoming damage while active (frostbite 1.1)
//   immobilize         — zeroes the target's velocity (root, stun)
//   suppress_abilities — blocks the target's ability casts (stun; consumer lands with enemy AI)
//   removed_by_tags    — a DamageEvent with one of these tags clears the effect (fire↔frost)
//   removes_on_apply   — applying this effect clears these other effects on the same target
//   hooks              — escape hatch for truly code-driven effects; EMPTY for all six built-ins,
//                        wired to a StatusHookRegistry only when the first such effect lands.
//
// Adding a new element that cancels an existing one edits only the new effect's data — no code,
// no change to any existing effect file (architecture-plan §3.5).
//
// Interactions:
//   - status/components.rs: StatusEffectInstance carries the def_id for lookup.
//   - status/systems/tick.rs: reads `tick` to emit DamageEvents for DoTs.
//   - status/systems/cross_interact.rs: reads `removed_by_tags` for element cancellation.
//   - status/systems/resolve.rs (Phase 3C): folds move/damage/immobilize into actor modifiers.
//   - ability/systems/execute.rs: EffectSpec::ApplyStatus emits ApplyStatusEvent.

use bevy::prelude::*;
use crate::ability::assets::HookId;
use crate::core::def_library::{DefAsset, DefLibrary};
use crate::core::events::DamageTag;

pub type StatusEffectId = String;

/// Loaded from assets/status_effects/<id>.status.ron.
#[derive(Asset, TypePath, Debug, Clone, serde::Deserialize)]
pub struct StatusEffectDef {
    pub id: StatusEffectId,
    pub display_name: String,
    pub stacking: StackingRule,
    pub base_duration_secs: f32,
    /// Damage-over-time cadence. `None` = no periodic damage (pure debuff).
    #[serde(default)]
    pub tick: Option<TickSpec>,
    /// Velocity multiplier while active. 1.0 = no change (frostbite 0.8).
    #[serde(default = "one")]
    pub move_speed_mult: f32,
    /// Incoming-damage multiplier while active. 1.0 = no change (frostbite 1.1).
    #[serde(default = "one")]
    pub damage_taken_mult: f32,
    /// Zeroes the target's velocity while active (root, stun).
    #[serde(default)]
    pub immobilize: bool,
    /// Blocks the target's ability casts while active (stun). Consumer arrives with enemy AI (Phase 5).
    #[serde(default)]
    pub suppress_abilities: bool,
    /// A DamageEvent carrying one of these tags removes this effect (fire clears frostbite, …).
    #[serde(default)]
    pub removed_by_tags: Vec<DamageTag>,
    /// Applying this effect removes these other effects from the same target.
    #[serde(default)]
    pub removes_on_apply: Vec<StatusEffectId>,
    /// Escape hatch for code-driven effects. Empty for the six built-ins; resolved against a
    /// StatusHookRegistry only when the first code-driven status effect lands (Phase 4+).
    #[serde(default)]
    pub hooks: Vec<HookId>,
}

/// Neutral defaults for the multiplier fields so a RON file can omit them.
fn one() -> f32 {
    1.0
}

/// Damage-over-time spec: every `interval_secs`, deal `damage` tagged with `tags`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TickSpec {
    pub interval_secs: f32,
    pub damage: f32,
    pub tags: Vec<DamageTag>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub enum StackingRule {
    /// One instance maximum. Re-applying resets the duration timer. (Most effects.)
    RefreshOnReapply,
    /// Up to N simultaneous instances; each with its own timer.
    StackCapped(u8),
    /// Unlimited stacking (rare; use carefully).
    StackUnlimited,
}

/// Resource mapping StatusEffectId → Handle<StatusEffectDef>. A `DefLibrary<StatusEffectDef>`
/// (see core/def_library.rs); populated at startup from `StatusEffectDef::MANIFEST`, read by the
/// status systems. The `.status.ron` extension keeps the loader from colliding on plain `.ron`.
pub type StatusLibrary = DefLibrary<StatusEffectDef>;

impl DefAsset for StatusEffectDef {
    const EXTENSIONS: &'static [&'static str] = &["status.ron"];
    const MANIFEST: &'static [(&'static str, &'static str)] = &[
        ("bleed", "status_effects/bleed.status.ron"),
        ("blaze", "status_effects/blaze.status.ron"),
        ("frostbite", "status_effects/frostbite.status.ron"),
        ("holy_mark", "status_effects/holy_mark.status.ron"),
        ("root", "status_effects/root.status.ron"),
        ("stun", "status_effects/stun.status.ron"),
        // Phase 4 — Mage stance-swap effects (self-applied on entering a stance).
        ("boots_of_fire", "status_effects/boots_of_fire.status.ron"),
        ("ice_barrier", "status_effects/ice_barrier.status.ron"),
    ];
}

#[cfg(test)]
mod tests {
    //! Parse the real RON asset files through the same `ron::de` path the AssetLoader uses.
    use super::*;

    fn load(rel_path: &str) -> StatusEffectDef {
        let full = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel_path);
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("read {full}: {e}"));
        ron::de::from_bytes::<StatusEffectDef>(&bytes)
            .unwrap_or_else(|e| panic!("parse {rel_path}: {e}"))
    }

    #[test]
    fn bleed_parses_as_physical_dot() {
        let def = load("assets/status_effects/bleed.status.ron");
        assert_eq!(def.id, "bleed");
        assert_eq!(def.stacking, StackingRule::RefreshOnReapply);
        let tick = def.tick.expect("bleed has a DoT tick");
        assert_eq!(tick.interval_secs, 1.0);
        assert_eq!(tick.tags, vec![DamageTag::Physical]);
        assert!(def.removed_by_tags.is_empty(), "physical DoT — no element cancels it");
        assert_eq!(def.move_speed_mult, 1.0);
        assert_eq!(def.damage_taken_mult, 1.0);
    }

    #[test]
    fn blaze_ticks_fire_and_is_removed_by_frost() {
        let def = load("assets/status_effects/blaze.status.ron");
        assert_eq!(def.tick.unwrap().tags, vec![DamageTag::Fire]);
        assert_eq!(def.removed_by_tags, vec![DamageTag::Frost]);
    }

    #[test]
    fn frostbite_slows_amplifies_and_is_removed_by_fire() {
        let def = load("assets/status_effects/frostbite.status.ron");
        assert!(def.tick.is_none(), "frostbite is a debuff, not a DoT");
        assert_eq!(def.move_speed_mult, 0.8);
        assert_eq!(def.damage_taken_mult, 1.1);
        assert_eq!(def.removed_by_tags, vec![DamageTag::Fire]);
    }

    #[test]
    fn root_and_stun_immobilize() {
        let root = load("assets/status_effects/root.status.ron");
        assert!(root.immobilize);
        assert!(!root.suppress_abilities, "root allows casting");
        let stun = load("assets/status_effects/stun.status.ron");
        assert!(stun.immobilize);
        assert!(stun.suppress_abilities, "stun locks casting too");
    }

    #[test]
    fn holy_mark_is_a_neutral_marker() {
        let def = load("assets/status_effects/holy_mark.status.ron");
        assert!(def.tick.is_none());
        assert_eq!(def.move_speed_mult, 1.0);
        assert!(def.removed_by_tags.is_empty());
        assert!(def.hooks.is_empty());
    }
}
