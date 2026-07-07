// resolve_params — the central modifier-stack evaluation.
//
// Called by ability/systems/execute.rs before each ability fires. The result is thrown away
// after use (not cached) — the talent list is short enough (~tens of entries at max
// progression) that recomputing per cast is negligible.
//
// Modifier application order (architecture-plan §3.4):
//   1. Collect all Modifier-effect talents whose ability_scope matches this ability or is None.
//      A Stack(N) talent taken `count` times contributes its modifier `count` times.
//   2. Per stat:
//        additive   = sum of all Add(v) values
//        mult_bonus = sum of all MultiplyAdd(v) values
//        resolved   = (base + additive) * (1.0 + mult_bonus)
//   3. Apply Override(v) last — replaces the stat outright (last override wins).
//
// Room modifiers (ThroneRoom curses, Phase 7) reuse the same StatModifier type and are passed
// via `extra_modifiers`; Phase 2 always passes an empty slice.
//
// Split into two functions:
//   resolve_params  — ECS-facing: resolves talent ids to defs and gathers their modifiers.
//   apply_modifiers — pure math core (no ECS, no assets); unit-tested directly.
//
// Universal stat baseline (Phase 9.1, §8.1(4) / §3.11 of the phase-9 plan): crit_chance, crit_mult,
// and attack_speed are seeded into every ability's resolved params even when its own RON never
// declares them, so a general (`ability_scope: None`) passive talent can reach every ability's crit/
// attack-speed the same way it reaches any other global stat. Neutral by construction — crit_chance
// defaults to 0.0 (ability/effects.rs never rolls the crit RNG when it is <= 0.0, so no shipped
// content perturbs the golden master's RNG stream) and attack_speed defaults to 0.0 (identity in the
// `cooldown / (1 + attack_speed)` formula, ability/systems/execute.rs).

use std::collections::HashMap;
use bevy::asset::Assets;
use crate::ability::assets::{AbilityId, StatId};
use crate::ability::behavior::ResolvedParams;
use crate::talent::assets::{ModOp, StatModifier, TalentDef, TalentEffect, TalentLibrary};
use crate::talent::components::AcquiredTalents;

/// Resolves the effective params for `ability_id` given the player's talent list.
///
/// - `base_params` comes from AbilityDef.base_params.
/// - `acquired` is the player's AcquiredTalents component.
/// - `talent_defs` / `talent_library` resolve each acquired TalentId to its loaded TalentDef.
///   Talents whose def isn't loaded yet (or has no RON file) are silently skipped.
/// - `extra_modifiers` are additional modifiers to apply (room curses, Phase 7); empty in Phase 2.
///
/// Only `TalentEffect::Modifier` talents contribute here; `Behavior` talents are handled via
/// ActiveHooks at execution time and do not touch the stat stack.
pub fn resolve_params(
    ability_id: &AbilityId,
    base_params: &HashMap<StatId, f32>,
    acquired: &AcquiredTalents,
    talent_defs: &Assets<TalentDef>,
    talent_library: &TalentLibrary,
    extra_modifiers: &[StatModifier],
) -> ResolvedParams {
    let mut modifiers: Vec<&StatModifier> = Vec::new();

    for (talent_id, count) in &acquired.entries {
        let Some(def) = talent_library.get(talent_id).and_then(|h| talent_defs.get(h)) else {
            continue; // not loaded / no RON file — skip gracefully
        };
        // Scope filter: applies to this ability specifically, or globally (None).
        let in_scope = match &def.ability_scope {
            None => true,
            Some(scope) => scope == ability_id,
        };
        if !in_scope {
            continue;
        }
        if let TalentEffect::Modifier(m) = &def.effect {
            // A Stack(N) talent taken `count` times contributes its modifier `count` times.
            for _ in 0..(*count).max(1) {
                modifiers.push(m);
            }
        }
    }

    modifiers.extend(extra_modifiers.iter());

    apply_modifiers(base_params, &modifiers)
}

/// Universal per-cast stats every ability resolves, whether or not its own RON declares them, so a
/// general passive talent (`ability_scope: None`) can modify crit/attack-speed uniformly. See the
/// module doc for why each default is neutral.
const UNIVERSAL_STAT_DEFAULTS: &[(&str, f32)] = &[
    ("crit_chance", 0.0),
    ("crit_mult", 2.0),
    ("attack_speed", 0.0),
];

/// Pure math core. Applies the additive / multiplicative / override stacks to `base_params` plus
/// the universal stat baseline. Iterates the combined stat key set — modifiers targeting a stat
/// neither the ability nor the universal baseline defines have no base to act on and are ignored.
fn apply_modifiers(base_params: &HashMap<StatId, f32>, modifiers: &[&StatModifier]) -> ResolvedParams {
    let mut additive: HashMap<&str, f32> = HashMap::new();
    let mut multiplicative: HashMap<&str, f32> = HashMap::new();
    let mut overrides: HashMap<&str, f32> = HashMap::new();

    for m in modifiers {
        match m.op {
            ModOp::Add(v) => *additive.entry(m.stat.as_str()).or_insert(0.0) += v,
            ModOp::MultiplyAdd(v) => *multiplicative.entry(m.stat.as_str()).or_insert(0.0) += v,
            ModOp::Override(v) => {
                overrides.insert(m.stat.as_str(), v); // last override wins
            }
        }
    }

    let mut resolved: HashMap<StatId, f32> = HashMap::with_capacity(base_params.len() + UNIVERSAL_STAT_DEFAULTS.len());
    let combined = base_params.iter().map(|(k, v)| (k.as_str(), *v)).chain(
        UNIVERSAL_STAT_DEFAULTS
            .iter()
            .filter(|(stat, _)| !base_params.contains_key(*stat))
            .copied(),
    );
    for (stat, base) in combined {
        let value = if let Some(o) = overrides.get(stat) {
            *o
        } else {
            let add = additive.get(stat).copied().unwrap_or(0.0);
            let mult = multiplicative.get(stat).copied().unwrap_or(0.0);
            (base + add) * (1.0 + mult)
        };
        resolved.insert(stat.to_string(), value);
    }

    ResolvedParams(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(pairs: &[(&str, f32)]) -> HashMap<StatId, f32> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    fn m(stat: &str, op: ModOp) -> StatModifier {
        StatModifier { stat: stat.to_string(), op }
    }

    #[test]
    fn identity_when_no_modifiers() {
        let b = base(&[("damage", 10.0), ("range", 60.0)]);
        let r = apply_modifiers(&b, &[]);
        assert_eq!(r.get("damage"), 10.0);
        assert_eq!(r.get("range"), 60.0);
    }

    #[test]
    fn additive_then_multiplicative() {
        let b = base(&[("damage", 10.0)]);
        // (10 + 5) * (1 + 0.5) = 22.5
        let add = m("damage", ModOp::Add(5.0));
        let mul = m("damage", ModOp::MultiplyAdd(0.5));
        let r = apply_modifiers(&b, &[&add, &mul]);
        assert!((r.get("damage") - 22.5).abs() < 1e-6);
    }

    #[test]
    fn multiply_add_stacks_additively() {
        let b = base(&[("leech_percent", 5.0)]);
        // three copies of +20% multiplicative → 5 * (1 + 0.6) = 8.0 (matches the RON comment)
        let mul = m("leech_percent", ModOp::MultiplyAdd(0.20));
        let r = apply_modifiers(&b, &[&mul, &mul, &mul]);
        assert!((r.get("leech_percent") - 8.0).abs() < 1e-6);
    }

    #[test]
    fn override_wins_over_add_and_mult() {
        let b = base(&[("cooldown", 1.2)]);
        let add = m("cooldown", ModOp::Add(10.0));
        let mul = m("cooldown", ModOp::MultiplyAdd(2.0));
        let ovr = m("cooldown", ModOp::Override(0.5));
        let r = apply_modifiers(&b, &[&add, &mul, &ovr]);
        assert!((r.get("cooldown") - 0.5).abs() < 1e-6);
    }

    #[test]
    fn modifier_on_absent_stat_is_ignored() {
        let b = base(&[("damage", 10.0)]);
        let stray = m("nonexistent", ModOp::Add(100.0));
        let r = apply_modifiers(&b, &[&stray]);
        assert_eq!(r.get("damage"), 10.0);
        assert_eq!(r.get("nonexistent"), 0.0); // not present → ResolvedParams::get returns 0
    }

    #[test]
    fn universal_stats_default_neutral_when_no_ability_or_talent_touches_them() {
        // No ability RON declares crit_chance/crit_mult/attack_speed today — resolve_params must
        // still surface neutral defaults so a *future* general talent has something to modify.
        let b = base(&[("damage", 10.0), ("cooldown", 1.2)]);
        let r = apply_modifiers(&b, &[]);
        assert_eq!(r.get("crit_chance"), 0.0, "no crit unless a talent grants it");
        assert_eq!(r.get("crit_mult"), 2.0, "sensible default crit multiplier");
        assert_eq!(r.get("attack_speed"), 0.0, "identity in cooldown / (1 + attack_speed)");
    }

    #[test]
    fn a_general_talent_reaches_the_universal_crit_chance_stat_on_any_ability() {
        // A general (ability_scope: None) "gain X% crit strike" passive must land on an ability
        // whose own RON never mentions crit_chance at all (architecture-plan §3.4's global-scope
        // promise, extended to the universal baseline).
        let b = base(&[("damage", 10.0)]);
        let general_crit = m("crit_chance", ModOp::Add(15.0));
        let r = apply_modifiers(&b, &[&general_crit]);
        assert_eq!(r.get("crit_chance"), 15.0);
    }

    #[test]
    fn an_ability_declared_universal_stat_overrides_the_default() {
        // If an ability's own RON *does* declare one of the universal stats (a bespoke crit-focused
        // ability), its base value wins over the generic default — the merge never clobbers it.
        let b = base(&[("crit_chance", 25.0)]);
        let r = apply_modifiers(&b, &[]);
        assert_eq!(r.get("crit_chance"), 25.0);
    }

    // Exercises the full ECS-facing path: acquired id → TalentLibrary → Assets<TalentDef> →
    // Modifier → apply_modifiers. Uses an in-memory Assets<TalentDef> (no AssetServer needed).
    #[test]
    fn resolve_params_applies_acquired_talent_through_assets() {
        use crate::talent::assets::{TalentEffect, TalentRarity, UniquenessConstraint};

        let mut assets = Assets::<TalentDef>::default();
        let leech = TalentDef {
            id: "death_strike_leech_common".to_string(),
            display_name: "Improved Leech".to_string(),
            ability_scope: Some("death_strike".to_string()),
            rarity: TalentRarity::Common,
            uniqueness: UniquenessConstraint::Stack(3),
            effect: TalentEffect::Modifier(StatModifier {
                stat: "leech_percent".to_string(),
                op: ModOp::MultiplyAdd(0.20),
            }),
        };
        let handle = assets.add(leech);
        let mut library = TalentLibrary::default();
        library.defs.insert("death_strike_leech_common".to_string(), handle);

        let base = base(&[("leech_percent", 5.0), ("damage", 10.0)]);

        // Two copies of the +20% leech talent → 5 * (1 + 0.4) = 7.0. Damage untouched.
        let mut acquired = AcquiredTalents::default();
        acquired.add("death_strike_leech_common".to_string());
        acquired.add("death_strike_leech_common".to_string());

        let ds = "death_strike".to_string();
        let resolved = resolve_params(&ds, &base, &acquired, &assets, &library, &[]);
        assert!((resolved.get("leech_percent") - 7.0).abs() < 1e-6);
        assert_eq!(resolved.get("damage"), 10.0);

        // The same talent must NOT affect a different ability (scope filter).
        let other = "frostbolt".to_string();
        let resolved_other = resolve_params(&other, &base, &acquired, &assets, &library, &[]);
        assert_eq!(resolved_other.get("leech_percent"), 5.0, "scoped talent doesn't leak");
    }
}
