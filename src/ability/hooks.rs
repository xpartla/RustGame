// Code-driven ability hooks (Phase 6) — the behavior-rewriting extension point for talents.
//
// The `BehaviorRegistry` (behavior.rs) decides an ability's *shape*; a `HookRegistry` here lets a
// TALENT rewrite one ability's cast without touching that ability's — or the talent's — base code
// (architecture-plan §3.4/§4). A `Behavior(hook_id)` talent installs an `ActiveHook` on the player
// when acquired (talent/systems/apply.rs, maintained since Phase 2 but never consumed until now);
// `execute_ready_abilities` runs a hook listed in `AbilityDef.hooks` ONLY when the caster carries
// that `ActiveHook` AND the hook is registered here — so an un-acquired or not-yet-implemented hook
// is zero-cost. This is the first code-driven hook, and pays the §8.5 "execute split" debt: the
// resolve → (Pre hooks) → behavior/apply → (Post hooks) boundary is now explicit.
//
// Adding a behavior-rewriting talent:
//   1. Write the TalentDef RON with `effect: Behavior("my_hook")`.
//   2. List it on the target ability's `hooks: [(Pre|Post, "my_hook")]`.
//   3. Implement AbilityHook + register it in AbilityPlugin::build.
//   (Numeric-only talents need none of this — they are pure `Modifier` data handled by the stack.)
//
// Execution model (deliberately &mut World-free, mirroring AbilityBehavior): a hook reads a
// `HookContext` (caster + zone presence) and either mutates the resolved params (Pre, before the
// behavior resolves) or reacts to the cast outcome (Post, after effects apply). It touches no
// Commands/EventWriter directly.

use bevy::prelude::*;
use std::collections::HashMap;
use crate::ability::assets::HookId;
use crate::ability::behavior::{CastOutcome, ResolvedParams};
use crate::core::components::Health;
use crate::zone::components::PlayerZonePresence;

/// Read-only view handed to a hook. Kept minimal; grows only as hooks demand more context.
pub struct HookContext<'a> {
    /// The casting entity.
    pub caster: Entity,
    /// The player's live zone-presence snapshot (zone/systems/presence.rs). The first hook —
    /// `blood_boil_dnd_range` — reads it to check `is_inside("death_and_decay")`.
    pub zones: &'a PlayerZonePresence,
    /// The caster's own `Health` (Phase 9.2) — Heart Strike's innate missing-health damage scaling
    /// and its execute-bonus talent both read the caster's own current/max.
    pub health: &'a Health,
}

/// A behavior-rewriting hook, referenced by `HookId` from `AbilityDef.hooks`. Both methods default
/// to no-ops so a hook implements only the phase it needs.
pub trait AbilityHook: Send + Sync + 'static {
    /// Runs BEFORE the behavior resolves; may mutate the resolved params (e.g. double a radius).
    fn pre(&self, _ctx: &HookContext, _params: &mut ResolvedParams) {}
    /// Runs AFTER effects apply; may react to what the cast hit (e.g. count kills). Read-only for now.
    fn post(&self, _ctx: &HookContext, _outcome: &CastOutcome) {}
}

/// Resource: `HookId → boxed hook`. Populated at plugin build; read-only at runtime. Mirrors
/// `BehaviorRegistry`. A `HookId` listed on an ability but absent here (e.g. `bone_shield_on_kill`,
/// whose shield system is deferred — §8.1(5)) is skipped silently: an expected in-progress state
/// during content buildout, unlike a missing *behavior* (which warns, because it kills the ability).
#[derive(Resource, Default)]
pub struct HookRegistry {
    hooks: HashMap<HookId, Box<dyn AbilityHook>>,
}

impl HookRegistry {
    pub fn register(&mut self, id: impl Into<HookId>, hook: impl AbilityHook) {
        self.hooks.insert(id.into(), Box::new(hook));
    }
    pub fn get(&self, id: &str) -> Option<&dyn AbilityHook> {
        self.hooks.get(id).map(|h| h.as_ref())
    }
}

// ── Built-in hooks ───────────────────────────────────────────────────────────────────────

/// "Empowered Reach" (the `blood_boil_dnd_range_rare` talent): Blood Boil has double range when cast
/// inside D&D (Mechanics). A Pre hook that doubles Blood Boil's `radius` param while the caster
/// stands in a `death_and_decay` zone. No D&D or base Blood Boil code is touched — the entire zone
/// interaction is these few lines (architecture-plan §4 "Talent 3 — Zone-interaction").
pub struct BloodBoilDndRange;

impl AbilityHook for BloodBoilDndRange {
    fn pre(&self, ctx: &HookContext, params: &mut ResolvedParams) {
        if ctx.zones.is_inside("death_and_decay") {
            params.scale("radius", 2.0);
        }
    }
}

/// Heart Strike's own identity (Phase 9.2; Mechanics/the BDK class blurb: "Increase damage as
/// health lowers"): scales `damage` up as the caster's current health drops, linearly to +100% at
/// 0 hp. An INNATE hook (`heart_strike.ability.ron`'s `innate_hooks`, not gated by `ActiveHooks`) —
/// this is base kit identity, not a talent.
pub struct HeartStrikeMissingHealth;

impl AbilityHook for HeartStrikeMissingHealth {
    fn pre(&self, ctx: &HookContext, params: &mut ResolvedParams) {
        let missing_frac = 1.0 - (ctx.health.current / ctx.health.max).clamp(0.0, 1.0);
        params.scale("damage", 1.0 + missing_frac);
    }
}

/// "Execute" (the `heart_strike_execute_epic` talent): +50% damage while under 25% health.
pub struct HeartStrikeExecuteBonus;

impl AbilityHook for HeartStrikeExecuteBonus {
    fn pre(&self, ctx: &HookContext, params: &mut ResolvedParams) {
        if ctx.health.max > 0.0 && ctx.health.current / ctx.health.max < 0.25 {
            params.scale("damage", 1.5);
        }
    }
}

/// The `bdk_passive_dnd_damage_boost` class passive (epic, unique — Mechanics: "Your damage is
/// lowered by 60%, your damage inside D&D is increased by 500%"). Listed on every BDK damage
/// ability's talent-gated `hooks` (death_strike, heart_strike, blood_boil); only fires with the
/// talent acquired. Net: ×0.4 outside D&D, ×0.4×6.0 = ×2.4 inside it.
pub struct BdkDndDamageBoost;

impl AbilityHook for BdkDndDamageBoost {
    fn pre(&self, ctx: &HookContext, params: &mut ResolvedParams) {
        params.scale("damage", 0.4);
        if ctx.zones.is_inside("death_and_decay") {
            params.scale("damage", 6.0);
        }
    }
}

/// The `bdk_passive_no_heal_cap` class passive (epic, unique — Mechanics: "You can no longer heal
/// above 35% max health, your leech is increased by 50%"). Only the leech half lives here — a Pre
/// hook scaling `leech_percent` ×1.5, listed on death_strike/blood_boil's talent-gated `hooks`. The
/// 35%-cap-enforcement half is a separate, always-running system
/// (`talent::systems::passives::enforce_heal_cap`) since it must clamp EVERY heal source (pickups,
/// D&D regen), not just this ability's own cast — outside what a per-cast Pre/Post hook can reach.
pub struct BdkNoHealCapLeechBoost;

impl AbilityHook for BdkNoHealCapLeechBoost {
    fn pre(&self, _ctx: &HookContext, params: &mut ResolvedParams) {
        params.scale("leech_percent", 1.5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(pairs: &[(&str, f32)]) -> ResolvedParams {
        ResolvedParams(pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect())
    }

    fn presence(zones: &[&str]) -> PlayerZonePresence {
        let mut p = PlayerZonePresence::default();
        for z in zones {
            p.active_zone_types.insert(z.to_string());
        }
        p
    }

    fn health(current: f32, max: f32) -> Health {
        Health { current, max }
    }

    #[test]
    fn blood_boil_dnd_range_doubles_radius_only_inside_dnd() {
        let hook = BloodBoilDndRange;
        let caster = Entity::from_raw(1);
        let hp = health(100.0, 100.0);

        // Outside every zone: radius unchanged.
        let mut p = params(&[("radius", 90.0)]);
        hook.pre(&HookContext { caster, zones: &presence(&[]), health: &hp }, &mut p);
        assert_eq!(p.get("radius"), 90.0, "no zone → no change");

        // Inside D&D: radius doubled.
        let mut p = params(&[("radius", 90.0)]);
        hook.pre(&HookContext { caster, zones: &presence(&["death_and_decay"]), health: &hp }, &mut p);
        assert_eq!(p.get("radius"), 180.0, "inside D&D → ×2 radius");

        // A different zone type doesn't trigger it.
        let mut p = params(&[("radius", 90.0)]);
        hook.pre(&HookContext { caster, zones: &presence(&["consecrated_ground"]), health: &hp }, &mut p);
        assert_eq!(p.get("radius"), 90.0, "wrong zone → no change");
    }

    #[test]
    fn heart_strike_missing_health_scales_up_to_double_at_zero_hp() {
        let hook = HeartStrikeMissingHealth;
        let caster = Entity::from_raw(1);
        let zones = presence(&[]);

        // Full health: no bonus.
        let mut p = params(&[("damage", 10.0)]);
        hook.pre(&HookContext { caster, zones: &zones, health: &health(100.0, 100.0) }, &mut p);
        assert_eq!(p.get("damage"), 10.0, "no missing health → no bonus");

        // Half health: +50%.
        let mut p = params(&[("damage", 10.0)]);
        hook.pre(&HookContext { caster, zones: &zones, health: &health(50.0, 100.0) }, &mut p);
        assert!((p.get("damage") - 15.0).abs() < 1e-6, "50% missing → +50% damage");

        // Zero health (about to die): +100%.
        let mut p = params(&[("damage", 10.0)]);
        hook.pre(&HookContext { caster, zones: &zones, health: &health(0.0, 100.0) }, &mut p);
        assert!((p.get("damage") - 20.0).abs() < 1e-6, "0 hp → double damage");
    }

    #[test]
    fn heart_strike_execute_bonus_only_below_25_percent() {
        let hook = HeartStrikeExecuteBonus;
        let caster = Entity::from_raw(1);
        let zones = presence(&[]);

        let mut p = params(&[("damage", 10.0)]);
        hook.pre(&HookContext { caster, zones: &zones, health: &health(26.0, 100.0) }, &mut p);
        assert_eq!(p.get("damage"), 10.0, "above the 25% threshold → no bonus");

        let mut p = params(&[("damage", 10.0)]);
        hook.pre(&HookContext { caster, zones: &zones, health: &health(24.0, 100.0) }, &mut p);
        assert!((p.get("damage") - 15.0).abs() < 1e-6, "below 25% → +50%");
    }

    #[test]
    fn bdk_no_heal_cap_leech_boost_scales_leech_by_1_5x() {
        let hook = BdkNoHealCapLeechBoost;
        let caster = Entity::from_raw(1);
        let hp = health(100.0, 100.0);

        let mut p = params(&[("leech_percent", 10.0)]);
        hook.pre(&HookContext { caster, zones: &presence(&[]), health: &hp }, &mut p);
        assert!((p.get("leech_percent") - 15.0).abs() < 1e-6, "×1.5 leech");
    }

    #[test]
    fn bdk_dnd_damage_boost_nets_negative_outside_and_positive_inside() {
        let hook = BdkDndDamageBoost;
        let caster = Entity::from_raw(1);
        let hp = health(100.0, 100.0);

        let mut p = params(&[("damage", 10.0)]);
        hook.pre(&HookContext { caster, zones: &presence(&[]), health: &hp }, &mut p);
        assert!((p.get("damage") - 4.0).abs() < 1e-6, "outside D&D: -60% (×0.4)");

        let mut p = params(&[("damage", 10.0)]);
        hook.pre(&HookContext { caster, zones: &presence(&["death_and_decay"]), health: &hp }, &mut p);
        assert!((p.get("damage") - 24.0).abs() < 1e-6, "inside D&D: ×0.4×6.0 = ×2.4");
    }
}
