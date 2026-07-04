// resolve_params — pure function, no ECS access.
//
// This is the central modifier-stack evaluation. Called by ability/systems/execute.rs
// before each ability fires. The result is thrown away after use (not cached) — the
// talent list is short enough that recomputing is negligible.
//
// Modifier application order:
//   1. Collect all Modifier talents whose ability_scope matches this ability or is None.
//   2. Per stat:
//      additive    = sum of all Add(v) values
//      mult_bonus  = sum of all MultiplyAdd(v) values
//      resolved    = (base + additive) * (1.0 + mult_bonus)
//   3. Apply Override(v) last — replaces resolved entirely.
//
// Room modifiers (ThroneRoom curses) use the same StatModifier type and are appended
// to the modifier list before evaluation. See world/graph.rs RoomModifierDef.

use std::collections::HashMap;
use crate::ability::assets::{AbilityId, StatId};
use crate::ability::behavior::ResolvedParams;
use crate::talent::assets::{ModOp, StatModifier, TalentDef, TalentEffect};
use crate::talent::components::AcquiredTalents;

/// Resolves the effective params for `ability_id` given the player's talent list.
/// `base_params` comes from AbilityDef.base_params.
/// `extra_modifiers` is used for room modifiers (ThroneRoom curses) passed as additional context.
///
/// TODO(Phase 2): implement once TalentDef assets are loadable.
pub fn resolve_params(
    ability_id: &AbilityId,
    base_params: &HashMap<StatId, f32>,
    acquired: &AcquiredTalents,
    _talent_defs: &bevy::asset::Assets<TalentDef>,
    extra_modifiers: &[StatModifier],
) -> ResolvedParams {
    let mut additive: HashMap<StatId, f32> = HashMap::new();
    let mut multiplicative: HashMap<StatId, f32> = HashMap::new();
    let mut overrides: HashMap<StatId, f32> = HashMap::new();

    // TODO(Phase 2): iterate acquired.entries, look up TalentDef, filter by ability_scope,
    // accumulate modifiers. For now, skeleton:
    let _ = (acquired, extra_modifiers);
    let _ = ability_id;

    let mut resolved: HashMap<StatId, f32> = base_params.clone();
    for (stat, base) in base_params {
        let add = additive.get(stat.as_str()).copied().unwrap_or(0.0);
        let mult = multiplicative.get(stat.as_str()).copied().unwrap_or(0.0);
        let value = (base + add) * (1.0 + mult);
        resolved.insert(stat.clone(), overrides.get(stat.as_str()).copied().unwrap_or(value));
    }

    ResolvedParams(resolved)
}
