// resolve_params — pure function; no ECS access.
//
// Takes the base params from AbilityDef and the player's AcquiredTalents list and
// produces a ResolvedParams map ready for the behavior and hooks to consume.
//
// Modifier application order (matching the architecture plan §3.4):
//   1. Sum all Add(f32) modifiers scoped to this ability or None (global).
//   2. Sum all MultiplyAdd(f32) modifiers for the same scope.
//   3. resolved = (base + additive_sum) * (1.0 + multiply_add_sum)
//   4. Apply Override(f32) last (replaces the stat outright).
//
// This is a pure function with no side effects — call it freely from any system.
// The result is not cached; regenerate it on each ability execution. The talent list
// is short enough (~40 entries at max progression) that this is negligible.
//
// Interactions:
//   - Called by ability/systems/execute.rs before behavior dispatch.
//   - Reads AcquiredTalents from talent/components.rs.
//   - Reads TalentDef assets to resolve effect type and scope.

use crate::ability::assets::AbilityId;
use crate::ability::behavior::ResolvedParams;

/// TODO(Phase 2): implement once TalentDef assets and AcquiredTalents are in place.
/// Signature will be approximately:
///   pub fn resolve_params(
///       ability_id: &AbilityId,
///       base_params: &HashMap<StatId, f32>,
///       acquired: &AcquiredTalents,
///       talent_defs: &Assets<TalentDef>,
///   ) -> ResolvedParams
pub fn resolve_params(_ability_id: &AbilityId) -> ResolvedParams {
    todo!("Phase 2")
}
