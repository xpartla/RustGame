// Element cancellation: fire removes frostbite, frost removes blaze (and any future pairing).
//
// Listens to DamageEvent (the `tags` field added in Phase 0) and, for each damaged target,
// emits a RemoveStatusEvent for every active effect whose StatusEffectDef.removed_by_tags
// intersects the event's tags. Because the rules live in the RON files, adding a new element
// that cancels an existing effect is data-only — no change here.
//
// This also covers DoT-driven cancellation: a blaze tick emits a Fire-tagged DamageEvent, which
// clears frostbite on the same target with no special case. Runs in StatusSet::CrossInteract,
// after StatusSet::Tick (so this frame's DoT ticks are included), before remove_status_effects.

use bevy::prelude::*;
use std::collections::HashSet;
use crate::core::events::DamageEvent;
use crate::status::assets::{StatusEffectDef, StatusLibrary};
use crate::status::components::{RemoveStatusEvent, StatusEffectInstance};

pub fn apply_cross_interactions(
    mut damage_events: EventReader<DamageEvent>,
    instances: Query<&StatusEffectInstance>,
    library: Res<StatusLibrary>,
    defs: Res<Assets<StatusEffectDef>>,
    mut remove_events: EventWriter<RemoveStatusEvent>,
) {
    // (target, effect_id) already scheduled for removal — avoids emitting duplicates when a
    // target takes several tagged hits in one frame.
    let mut emitted: HashSet<(Entity, String)> = HashSet::new();

    for ev in damage_events.read() {
        if ev.tags.is_empty() {
            continue;
        }
        for inst in &instances {
            if inst.target != ev.target {
                continue;
            }
            let key = (ev.target, inst.def_id.clone());
            if emitted.contains(&key) {
                continue;
            }
            let Some(def) = library.get(&inst.def_id).and_then(|h| defs.get(h)) else {
                continue;
            };
            if def.removed_by_tags.iter().any(|t| ev.tags.contains(t)) {
                remove_events.write(RemoveStatusEvent {
                    target: ev.target,
                    effect_id: inst.def_id.clone(),
                });
                emitted.insert(key);
            }
        }
    }
}
