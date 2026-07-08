// Zone occupant-tick effects (Phase 6D; Phase 9.3 adds the optional slow + occupant-count scaling).
//
// Each zone with a `ZoneEffects` component applies, on a fixed ZONE_TICK_INTERVAL cadence:
//   - damage to every OPPOSING-faction actor standing inside (Consecrated Ground's Holy DoT),
//     optionally scaled up per additional occupant and/or paired with a slow status
//     (`consecrated_ground_count_scaling_rare` / `consecrated_ground_slow_common`), and
//   - regen to the zone's OWNER while it stands inside (D&D healing).
//
// Runs in CombatSet::Damage so the emitted DamageEvent/HealEvent resolve this frame, exactly like
// the melee cone, projectile impacts, and status DoTs. Damage flows through the shared apply_damage
// (so DamageTaken/DealtModifier + kill-credit apply for free); regen through apply_heal (clamped to
// max). No RNG → deterministic; a zone deals no damage until it has been alive one full tick.

use bevy::prelude::*;
use crate::core::components::{Faction, Health, WorldPosition};
use crate::core::events::{DamageEvent, DamageTag, HealEvent};
use crate::constants::CONSECRATED_COUNT_SCALING_FRACTION;
use crate::status::components::ApplyStatusEvent;
use crate::zone::components::{PersistentZone, ZoneEffects};

pub fn zone_tick_effects(
    time: Res<Time>,
    mut zones: Query<(&PersistentZone, &WorldPosition, &Faction, &mut ZoneEffects)>,
    // Candidate occupants — actors only, never other zones (which also carry WorldPosition+Faction).
    actors: Query<(Entity, &WorldPosition, &Faction), Without<PersistentZone>>,
    healths: Query<&Health>,
    mut damage_events: EventWriter<DamageEvent>,
    mut heal_events: EventWriter<HealEvent>,
    mut status_events: EventWriter<ApplyStatusEvent>,
) {
    for (zone, zone_pos, zone_faction, mut effects) in &mut zones {
        effects.tick.tick(time.delta());
        if !effects.tick.just_finished() {
            continue;
        }
        let center = zone_pos.0;

        // DoT to opposing-faction occupants (a ground AoE — centre distance vs. radius). The tag is
        // Holy (Consecrated Ground); a data-driven element joins when a non-Holy damage zone lands.
        if effects.damage_per_second > 0.0 {
            let opposing = zone_faction.opposing();
            let occupants: Vec<Entity> = actors
                .iter()
                .filter(|(_, pos, faction)| **faction == opposing && pos.0.distance(center) <= zone.radius)
                .map(|(e, _, _)| e)
                .collect();
            // "Deals X% increased damage per enemy inside" — scales off the pack size THIS tick.
            let scale = if effects.scales_with_occupants && occupants.len() > 1 {
                1.0 + (occupants.len() - 1) as f32 * CONSECRATED_COUNT_SCALING_FRACTION
            } else {
                1.0
            };
            for entity in occupants {
                damage_events.write(DamageEvent {
                    target: entity,
                    amount: effects.damage_per_second * scale,
                    source: zone.owner,
                    tags: vec![DamageTag::Holy],
                });
                if let Some(status) = &effects.slow_status {
                    status_events.write(ApplyStatusEvent {
                        target: entity,
                        source: zone.owner,
                        effect_id: status.clone(),
                        stacks: 1,
                    });
                }
            }
        }

        // Regen the owner while it stands inside (D&D heals X% of max health per second).
        if effects.regen_fraction > 0.0 {
            if let Ok((_, owner_pos, _)) = actors.get(zone.owner) {
                if owner_pos.0.distance(center) <= zone.radius {
                    if let Ok(health) = healths.get(zone.owner) {
                        heal_events.write(HealEvent {
                            target: zone.owner,
                            amount: health.max * effects.regen_fraction,
                        });
                    }
                }
            }
        }
    }
}
