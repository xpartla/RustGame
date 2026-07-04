// resolve_params — pure function; no ECS access.
//
// Takes the base params from an AbilityDef and produces a ResolvedParams map ready for the
// behavior and hooks to consume.
//
// PHASE 1: talents do not exist yet, so the resolved params are the base params verbatim.
// PHASE 2 layers the AcquiredTalents modifier stack on top (architecture-plan §3.4):
//   1. Sum all Add(f32) modifiers scoped to this ability or None (global).
//   2. Sum all MultiplyAdd(f32) modifiers for the same scope.
//   3. resolved = (base + additive_sum) * (1.0 + multiply_add_sum)
//   4. Apply Override(f32) last (replaces the stat outright).
//
// The result is not cached; regenerate it on each ability execution. The talent list is short
// enough (~40 entries at max progression) that this is negligible.

use std::collections::HashMap;
use crate::ability::assets::StatId;
use crate::ability::behavior::ResolvedParams;

/// Phase 1: identity resolution (no modifiers). Signature will grow an `&AcquiredTalents` and
/// `&Assets<TalentDef>` argument in Phase 2.
pub fn resolve_params(base_params: &HashMap<StatId, f32>) -> ResolvedParams {
    ResolvedParams(base_params.clone())
}
