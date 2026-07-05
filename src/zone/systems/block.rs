// AMZ projectile blocking (Phase 6E).
//
// A `ZoneBlocksProjectiles` zone (AMZ, Friendly) destroys any projectile aimed at the side it
// protects (`payload.target_faction == zone.Faction`) while that projectile is inside the zone —
// UNLESS the projectile was emitted from inside (its source stands in the zone), per Mechanics:
// "if enemies emit projectiles from inside the zone it has no effect — it acts as a barrier."
//
// Runs in CombatSet::Damage, ordered AFTER move_projectiles (so it sees the shot's current
// position) and BEFORE projectile_collision (so a blocked shot never lands its payload).

use bevy::prelude::*;
use crate::core::components::{Faction, WorldPosition};
use crate::projectile::components::{ProjectileMotion, ProjectilePayload};
use crate::zone::components::{PersistentZone, ZoneBlocksProjectiles};

pub fn block_projectiles_in_zones(
    mut commands: Commands,
    zones: Query<(&PersistentZone, &WorldPosition, &Faction), With<ZoneBlocksProjectiles>>,
    projectiles: Query<(Entity, &WorldPosition, &ProjectilePayload), With<ProjectileMotion>>,
    positions: Query<&WorldPosition>,
) {
    for (proj_entity, proj_pos, payload) in &projectiles {
        for (zone, zone_pos, zone_faction) in &zones {
            // Only shots aimed at the side this zone protects (an enemy bolt targeting the Friendly
            // player, inside a Friendly AMZ). A player's own shot (targeting Hostiles) is ignored.
            if payload.target_faction != *zone_faction {
                continue;
            }
            if proj_pos.0.distance(zone_pos.0) > zone.radius {
                continue;
            }
            // Exception: emitted from inside the zone (source stands inside) → passes freely.
            if let Ok(src_pos) = positions.get(payload.source) {
                if src_pos.0.distance(zone_pos.0) <= zone.radius {
                    continue;
                }
            }
            commands.entity(proj_entity).try_despawn();
            break; // destroyed — no need to test further zones for this projectile
        }
    }
}
