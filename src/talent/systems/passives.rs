// Class-passive consumers that don't fit the per-cast Pre/Post hook pipeline (Phase 9.2, BDK's
// remaining `class_passive_pool` entries). Each reacts to `ActiveHooks`/`AcquiredTalents` directly,
// mirroring the shape of `ability::systems::bone_shield` and `ability::systems::purgatory`.

use bevy::prelude::*;
use crate::core::components::{BaseHealth, Health, HealingTakenModifier, LastHitBy};
use crate::core::events::HealEvent;
use crate::enemy::components::Enemy;
use crate::player::components::Player;
use crate::talent::components::{AcquiredTalents, ActiveHooks};

/// "You can no longer heal above 35% max health" (`bdk_passive_no_heal_cap`'s cap half — the
/// leech-boost half is the `bdk_no_heal_cap` Pre hook, ability/hooks.rs). A per-frame clamp rather
/// than an event-driven one so it catches every heal source uniformly (pickups, D&D regen, a future
/// merchant heal) without needing a hook on each one individually. Runs in CombatSet::Apply, after
/// `apply_heal`, so a heal landing this same frame is clamped immediately.
const HEAL_CAP_FRACTION: f32 = 0.35;

pub fn enforce_heal_cap(mut actors: Query<(&mut Health, &ActiveHooks)>) {
    for (mut health, hooks) in &mut actors {
        if !hooks.contains("bdk_no_heal_cap") {
            continue;
        }
        let cap = health.max * HEAL_CAP_FRACTION;
        if health.current > cap {
            health.current = cap;
        }
    }
}

/// "20% overkill damage is leeched" (`bdk_passive_overkill_leech`). Reads the dying enemy's
/// negative `Health.current` (the overkill amount `apply_damage` doesn't clamp at 0) and
/// `LastHitBy` for kill credit, healing the killer if their `ActiveHooks` carries the flag. Runs in
/// CombatSet::Death, `.before(enemy_death)` (talent/plugin.rs pin, Phase 9.5) — Bevy auto-inserts a
/// sync point right after `enemy_death`'s `Commands::despawn`, so without an explicit order this
/// read can lose a same-set tie-break and see the entity already gone (found when adding the
/// Mage's own two Death-set readers shifted that tie-break — see ability/plugin.rs's fuller note).
const OVERKILL_LEECH_FRACTION: f32 = 0.20;

pub fn overkill_leech_on_kill(
    mut heal_events: EventWriter<HealEvent>,
    dying: Query<(&Health, &LastHitBy), With<Enemy>>,
    killers: Query<&ActiveHooks>,
) {
    for (health, last_hit_by) in &dying {
        if health.current >= 0.0 {
            continue;
        }
        let Ok(hooks) = killers.get(last_hit_by.0) else { continue };
        if !hooks.contains("bdk_overkill_leech") {
            continue;
        }
        heal_events.write(HealEvent { target: last_hit_by.0, amount: -health.current * OVERKILL_LEECH_FRACTION });
    }
}

/// "Increase health by X% and healing taken by Y%" (`bdk_passive_health_and_healing`, Stack(3)).
/// Recomputes BOTH numbers from `BaseHealth` (the pristine, un-boosted max) every time
/// `AcquiredTalents` changes, so re-acquiring a stack never compounds against an already-boosted
/// value. The health bonus grants its `Health.max` delta to `Health.current` too (a mid-run pickup
/// shouldn't feel wasted), matching how a level-up implicitly full-heals elsewhere in this codebase.
const HEALTH_PCT_PER_STACK: f32 = 0.10;
const HEALING_TAKEN_PCT_PER_STACK: f32 = 0.15;

pub fn resolve_health_and_healing(
    mut commands: Commands,
    mut players: Query<
        (Entity, &AcquiredTalents, &BaseHealth, &mut Health, Option<&HealingTakenModifier>),
        (With<Player>, Changed<AcquiredTalents>),
    >,
) {
    for (entity, acquired, base, mut health, current_mod) in &mut players {
        let stacks = acquired.count_of("bdk_passive_health_and_healing") as f32;

        let new_max = base.0 * (1.0 + stacks * HEALTH_PCT_PER_STACK);
        let delta = new_max - health.max;
        health.max = new_max;
        if delta > 0.0 {
            health.current = (health.current + delta).min(new_max);
        } else {
            health.current = health.current.min(new_max);
        }

        let healing_mult = 1.0 + stacks * HEALING_TAKEN_PCT_PER_STACK;
        if (healing_mult - 1.0).abs() > 1e-6 {
            if current_mod.map(|m| m.0) != Some(healing_mult) {
                commands.entity(entity).insert(HealingTakenModifier(healing_mult));
            }
        } else if current_mod.is_some() {
            commands.entity(entity).remove::<HealingTakenModifier>();
        }
    }
}
