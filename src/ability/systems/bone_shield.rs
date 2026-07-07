// Bone Shield's kill-counter + grant (Phase 9.2, Death Strike epic talent).
//
// Mechanics: "After Death Strike kills X enemies, gain bone shield that blocks 1 next attack /
// projectile." Simplified here to "after the player's kills (from any source) reach X" — DamageEvent
// carries no ability provenance (only `source`/`tags`), so attributing a kill specifically to Death
// Strike (vs. Blood Boil, Heart Strike, a Companion minion, ...) isn't representable without adding
// that provenance everywhere. The kill-count *threshold* and the shield *amount* still come from
// death_strike's own (talent-modified) resolved params, keeping the numbers where the RON already
// declares them.
//
// Not wired through HookRegistry: `AbilityHook::post` is deliberately read-only (no
// Commands/EventWriter — see ability/hooks.rs's module doc), and this needs a persistent per-killer
// counter plus a conditional `GainShieldEvent`. Runs in CombatSet::Death (same set as enemy_death,
// unordered relative to it — both only read the dying enemy's Health/LastHitBy before Commands
// despawn it at end of frame).
//
// `BoneShieldProgress` (talent/components.rs) is inserted unconditionally alongside
// AcquiredTalents/ActiveHooks by `attach_talent_components` — every player has one from spawn,
// whether or not they ever take this talent — so there is no "first kill after acquiring is
// dropped while the component lands" race to reason about.
//
// Deliberately no `HashMap` batching by killer: std's default hasher is randomly reseeded from
// system entropy on every `HashMap::new()` (unlike `RunRng` or Bevy's own deterministic Query
// iteration), so an intermediate "kills this frame, by killer" map would make iteration order —
// and therefore write order into `shield_events`/`progress` — vary run-to-run even for an
// identical seed and script. Each death is instead processed as its own pass over `dying`, in
// the query's stable iteration order.

use bevy::prelude::*;
use crate::ability::assets::{AbilityDef, AbilityLibrary};
use crate::core::components::{Health, LastHitBy};
use crate::core::events::GainShieldEvent;
use crate::enemy::components::Enemy;
use crate::talent::assets::{TalentDef, TalentLibrary};
use crate::talent::components::{AcquiredTalents, ActiveHooks, BoneShieldProgress};
use crate::talent::modifier::resolve_params;

pub fn bone_shield_on_kill(
    mut shield_events: EventWriter<GainShieldEvent>,
    library: Res<AbilityLibrary>,
    defs: Res<Assets<AbilityDef>>,
    talent_defs: Res<Assets<TalentDef>>,
    talent_library: Res<TalentLibrary>,
    dying: Query<(&Health, &LastHitBy), With<Enemy>>,
    mut killers: Query<(&ActiveHooks, Option<&AcquiredTalents>, &mut BoneShieldProgress)>,
) {
    let Some(def) = library.get("death_strike").and_then(|h| defs.get(h)) else {
        return; // asset not loaded yet
    };

    for (health, last_hit_by) in &dying {
        if health.current > 0.0 {
            continue;
        }
        let Ok((hooks, acquired_opt, mut progress)) = killers.get_mut(last_hit_by.0) else {
            continue; // no talent bookkeeping at all (e.g. a Companion minion's kill)
        };
        if !hooks.contains("bone_shield_on_kill") {
            continue;
        }
        let no_talents = AcquiredTalents::default();
        let acquired = acquired_opt.unwrap_or(&no_talents);
        let params = resolve_params(&"death_strike".to_string(), &def.base_params, acquired, &talent_defs, &talent_library, &[]);
        let threshold = params.get("bone_shield_kill_threshold").max(1.0) as u32;
        let shield_amount = params.get("bone_shield_amount");

        progress.0 += 1;
        while progress.0 >= threshold {
            shield_events.write(GainShieldEvent { target: last_hit_by.0, amount: shield_amount });
            progress.0 -= threshold;
        }
    }
}
