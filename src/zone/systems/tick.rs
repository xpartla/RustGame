// Zone occupant-tick effects (Phase 6D).
//
// Each zone with a `ZoneEffects` component applies, on a fixed ZONE_TICK_INTERVAL cadence:
//   - damage to every OPPOSING-faction actor standing inside (Consecrated Ground's Holy DoT), and
//   - regen to the zone's OWNER while it stands inside (D&D healing).
//
// Runs in CombatSet::Damage so the emitted DamageEvent/HealEvent resolve this frame, exactly like
// the melee cone, projectile impacts, and status DoTs. Damage flows through the shared apply_damage
// (so DamageTaken/DealtModifier + kill-credit apply for free); regen through apply_heal (clamped to
// max). No RNG → deterministic; a zone deals no damage until it has been alive one full tick.

use bevy::prelude::*;
use crate::core::components::{Faction, Health, WorldPosition};
use crate::core::events::{DamageEvent, DamageTag, HealEvent};
use crate::zone::components::{PersistentZone, ZoneEffects};

pub fn zone_tick_effects(
    time: Res<Time>,
    mut zones: Query<(&PersistentZone, &WorldPosition, &Faction, &mut ZoneEffects)>,
    // Candidate occupants — actors only, never other zones (which also carry WorldPosition+Faction).
    actors: Query<(Entity, &WorldPosition, &Faction), Without<PersistentZone>>,
    healths: Query<&Health>,
    mut damage_events: EventWriter<DamageEvent>,
    mut heal_events: EventWriter<HealEvent>,
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
            for (entity, pos, faction) in &actors {
                if *faction == opposing && pos.0.distance(center) <= zone.radius {
                    damage_events.write(DamageEvent {
                        target: entity,
                        amount: effects.damage_per_second,
                        source: zone.owner,
                        tags: vec![DamageTag::Holy],
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
