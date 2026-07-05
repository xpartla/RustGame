// Applying status effects: consumes ApplyStatusEvent, spawns StatusEffectInstance entities
// honoring the effect's StackingRule.
//
//   RefreshOnReapply — at most one instance; re-application resets its duration timer.
//   StackCapped(n)   — up to n instances (each its own timer); extra applications are dropped.
//   StackUnlimited   — always spawns.
//
// `removes_on_apply` clears the listed effects from the same target first (none of the six
// built-ins use it; implemented for completeness). Runs at the head of StatusSet::Tick.

use bevy::prelude::*;
use crate::status::assets::{StackingRule, StatusEffectDef, StatusLibrary};
use crate::status::components::{ApplyStatusEvent, StatusEffectInstance};

pub fn apply_status_effects(
    mut events: EventReader<ApplyStatusEvent>,
    library: Res<StatusLibrary>,
    defs: Res<Assets<StatusEffectDef>>,
    mut commands: Commands,
    mut instances: Query<(Entity, &mut StatusEffectInstance)>,
) {
    for ev in events.read() {
        let Some(def) = library.get(&ev.effect_id).and_then(|h| defs.get(h)) else {
            continue; // unknown id / asset not loaded — skip gracefully
        };

        // Clear any effects this one displaces on the same target.
        if !def.removes_on_apply.is_empty() {
            for (e, inst) in instances.iter() {
                if inst.target == ev.target && def.removes_on_apply.contains(&inst.def_id) {
                    commands.entity(e).try_despawn();
                }
            }
        }

        let stacks = ev.stacks.max(1) as usize;
        match def.stacking {
            StackingRule::RefreshOnReapply => {
                let existing = instances
                    .iter_mut()
                    .find(|(_, i)| i.target == ev.target && i.def_id == ev.effect_id);
                if let Some((_, mut inst)) = existing {
                    inst.timer = Timer::from_seconds(def.base_duration_secs, TimerMode::Once);
                } else {
                    spawn_instance(&mut commands, ev.target, ev.source, def);
                }
            }
            StackingRule::StackCapped(cap) => {
                let current = instances
                    .iter()
                    .filter(|(_, i)| i.target == ev.target && i.def_id == ev.effect_id)
                    .count();
                let room = (cap as usize).saturating_sub(current);
                for _ in 0..room.min(stacks) {
                    spawn_instance(&mut commands, ev.target, ev.source, def);
                }
            }
            StackingRule::StackUnlimited => {
                for _ in 0..stacks {
                    spawn_instance(&mut commands, ev.target, ev.source, def);
                }
            }
        }
    }
}

fn spawn_instance(commands: &mut Commands, target: Entity, source: Entity, def: &StatusEffectDef) {
    let tick_timer = def
        .tick
        .as_ref()
        .map(|t| Timer::from_seconds(t.interval_secs, TimerMode::Repeating));
    commands.spawn(StatusEffectInstance {
        def_id: def.id.clone(),
        target,
        source,
        timer: Timer::from_seconds(def.base_duration_secs, TimerMode::Once),
        tick_timer,
    });
}
