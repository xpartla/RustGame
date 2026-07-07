// Purgatory's cheat-death interceptor (Phase 9.2, BDK band-4/6).
//
// Reacts to a lethal hit AFTER apply_damage has already applied it (Health.current may have gone
// negative — apply_damage doesn't clamp at 0) rather than trying to predict "would this hit be
// lethal" beforehand: correct for any combination of simultaneous hits in one frame, and needs no
// duplicate damage-scaling logic. Runs in CombatSet::Apply, ordered `.after(apply_damage)` and
// before CombatSet::Death, so the rescue lands before any death system would otherwise despawn
// the entity.
//
// Purgatory itself is never triggered through the normal TriggerAbilityEvent pipeline — its
// AbilityDef exists only so this system can read its (talent-modified) resolved params and manage
// its AbilityCooldown through the same generic tick_ability_cooldowns every other ability uses.

use bevy::prelude::*;
use crate::ability::assets::{AbilityDef, AbilityLibrary};
use crate::ability::components::{AbilityCooldown, AbilityInstance};
use crate::core::components::{Health, Invulnerable};
use crate::talent::assets::{TalentDef, TalentLibrary};
use crate::talent::components::AcquiredTalents;
use crate::talent::modifier::resolve_params;

pub fn purgatory_cheat_death(
    mut commands: Commands,
    library: Res<AbilityLibrary>,
    defs: Res<Assets<AbilityDef>>,
    talent_defs: Res<Assets<TalentDef>>,
    talent_library: Res<TalentLibrary>,
    mut victims: Query<(Entity, &mut Health, Option<&AcquiredTalents>), Without<Invulnerable>>,
    mut instances: Query<(&AbilityInstance, &mut AbilityCooldown)>,
) {
    for (entity, mut health, acquired_opt) in &mut victims {
        if health.current > 0.0 {
            continue;
        }
        let Some((instance, mut cooldown)) = instances
            .iter_mut()
            .find(|(i, cd)| i.owner == entity && i.def_id == "purgatory" && cd.is_ready())
        else {
            continue; // no ready Purgatory instance owned by this (about to die) entity
        };
        let Some(def) = library.get(&instance.def_id).and_then(|h| defs.get(h)) else {
            continue;
        };

        let no_talents = AcquiredTalents::default();
        let acquired = acquired_opt.unwrap_or(&no_talents);
        let params = resolve_params(&instance.def_id, &def.base_params, acquired, &talent_defs, &talent_library, &[]);

        let restore_frac = (params.get("restore_percent") / 100.0).clamp(0.0, 1.0);
        let immunity_secs = params.get("immunity_secs").max(0.0);

        // `.max(1.0)` guards against a degenerate 0-max Health computing a 0-hp "rescue" that
        // death systems would immediately re-kill.
        health.current = (health.max * restore_frac).max(1.0);
        commands.entity(entity).insert(Invulnerable(Timer::from_seconds(immunity_secs, TimerMode::Once)));

        cooldown.elapsed = 0.0;
        cooldown.duration = params.get("cooldown");
    }
}
