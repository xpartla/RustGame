// StatusEffectDef — one file per effect, loaded from assets/status_effects/*.ron.
//
// Cross-element interactions are encoded here, not in the ability that applies the effect.
// This means: adding a new element that cancels frostbite only requires editing
// frostbite.ron's `removed_by_tags` list. The frostbite system code is untouched.
//
// Key fields:
//   stacking         — governs how multiple applications interact (see StackingRule).
//   removed_by_tags  — this effect is removed when a DamageEvent with one of these tags hits.
//   removes_on_apply — applying this effect removes these other effects from the target.
//
// Known effects: bleed, blaze, frostbite, holy_mark, root, stun.
// Each lives in assets/status_effects/<id>.ron.
//
// Interactions:
//   - status/components.rs: StatusEffectInstance holds the def_id for lookup.
//   - status/systems/tick.rs: reads on_tick_hooks to emit DamageEvents for DoTs.
//   - status/systems/cross_interact.rs: reads removed_by_tags to handle element cancellation.
//   - ability/behavior.rs: behaviors apply status effects by emitting ApplyStatusEvent.

use bevy::prelude::*;
use crate::ability::assets::HookId;
use crate::core::events::DamageTag;

pub type StatusEffectId = String;

/// Loaded from assets/status_effects/<id>.ron.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct StatusEffectDef {
    pub id: StatusEffectId,
    pub display_name: String,
    pub stacking: StackingRule,
    pub base_duration_secs: f32,
    /// If Some, a damage tick fires at this interval using on_tick_hooks.
    pub tick_interval_secs: Option<f32>,
    /// Called on application: setup, VFX, sound.
    pub on_apply_hooks: Vec<HookId>,
    /// Called each tick: usually emits DamageEvent.
    pub on_tick_hooks: Vec<HookId>,
    /// Called on removal (expiry, element cancel, or talent consumption).
    pub on_remove_hooks: Vec<HookId>,
    /// This effect is removed when the target is hit by damage with one of these tags.
    /// Example: frostbite.ron has removed_by_tags: [Fire]
    pub removed_by_tags: Vec<DamageTag>,
    /// Applying this effect removes these other effects from the same target.
    /// Example: blaze.ron could list frostbite here to remove it on blaze application.
    /// (Cross-reference: use removed_by_tags on the other side too for mutual cancellation.)
    pub removes_on_apply: Vec<StatusEffectId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StackingRule {
    /// One instance maximum. Re-applying resets the duration timer. (Most effects.)
    RefreshOnReapply,
    /// Up to N simultaneous instances; each with its own timer.
    /// Example: bleed with Mega Bleed talent (unique[3]).
    StackCapped(u8),
    /// Unlimited stacking (rare; use carefully).
    StackUnlimited,
}
