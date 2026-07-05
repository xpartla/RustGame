// Ticking status effects: advances duration + DoT timers, emits DamageEvent for periodic
// effects (bleed, blaze), and despawns expired instances.
//
// A DoT tick emits a DamageEvent carrying the effect's `source` (for kill credit) and the
// TickSpec's element tags. Fire ticks therefore clear frostbite emergently, via cross_interact —
// no special case here. Runs in StatusSet::Tick, after apply_status_effects.
//
// DoT damage lands one frame later (it is emitted here, in Tick; apply_damage already ran this
// frame in CombatSet::Apply). This one-frame latency is deterministic and pinned by scenarios.

use bevy::prelude::*;
use crate::core::events::DamageEvent;
use crate::status::assets::{StatusEffectDef, StatusLibrary};
use crate::status::components::StatusEffectInstance;

pub fn tick_status_effects(
    time: Res<Time>,
    library: Res<StatusLibrary>,
    defs: Res<Assets<StatusEffectDef>>,
    mut commands: Commands,
    mut damage_events: EventWriter<DamageEvent>,
    mut instances: Query<(Entity, &mut StatusEffectInstance)>,
) {
    let dt = time.delta();
    for (entity, inst) in &mut instances {
        let inst = inst.into_inner(); // split-borrow tick_timer vs. the id/target/source fields

        // Damage-over-time.
        if let Some(tick_timer) = inst.tick_timer.as_mut() {
            tick_timer.tick(dt);
            let fires = tick_timer.times_finished_this_tick();
            if fires > 0 {
                if let Some(tick) = library
                    .get(&inst.def_id)
                    .and_then(|h| defs.get(h))
                    .and_then(|d| d.tick.as_ref())
                {
                    for _ in 0..fires {
                        damage_events.write(DamageEvent {
                            target: inst.target,
                            amount: tick.damage,
                            source: inst.source,
                            tags: tick.tags.clone(),
                        });
                    }
                }
            }
        }

        // Duration.
        inst.timer.tick(dt);
        if inst.timer.finished() {
            commands.entity(entity).try_despawn();
        }
    }
}
