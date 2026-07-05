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
use crate::zone::components::PlayerZonePresence;

/// Read-only view handed to a hook. Kept minimal; grows only as hooks demand more context.
pub struct HookContext<'a> {
    /// The casting entity.
    pub caster: Entity,
    /// The player's live zone-presence snapshot (zone/systems/presence.rs). The first hook —
    /// `blood_boil_dnd_range` — reads it to check `is_inside("death_and_decay")`.
    pub zones: &'a PlayerZonePresence,
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

    #[test]
    fn blood_boil_dnd_range_doubles_radius_only_inside_dnd() {
        let hook = BloodBoilDndRange;
        let caster = Entity::from_raw(1);

        // Outside every zone: radius unchanged.
        let mut p = params(&[("radius", 90.0)]);
        hook.pre(&HookContext { caster, zones: &presence(&[]) }, &mut p);
        assert_eq!(p.get("radius"), 90.0, "no zone → no change");

        // Inside D&D: radius doubled.
        let mut p = params(&[("radius", 90.0)]);
        hook.pre(&HookContext { caster, zones: &presence(&["death_and_decay"]) }, &mut p);
        assert_eq!(p.get("radius"), 180.0, "inside D&D → ×2 radius");

        // A different zone type doesn't trigger it.
        let mut p = params(&[("radius", 90.0)]);
        hook.pre(&HookContext { caster, zones: &presence(&["consecrated_ground"]) }, &mut p);
        assert_eq!(p.get("radius"), 90.0, "wrong zone → no change");
    }
}
