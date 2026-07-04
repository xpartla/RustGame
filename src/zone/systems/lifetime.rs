// Zone lifetime and anchor movement.
//
// Two systems:
//   tick_zone_lifetimes — advances PersistentZone.duration timers, despawns expired zones.
//   move_anchored_zones — for ZoneAnchor::Follow(entity), syncs the zone's WorldPosition
//                         to the followed entity's position each frame.
//                         (Used by the AMZ epic talent: zone attached to player as they move.)

use bevy::prelude::*;
use crate::zone::components::{PersistentZone, ZoneAnchor};
use crate::core::components::WorldPosition;

/// TODO(Phase 6): implement.
pub fn tick_zone_lifetimes(
    time: Res<Time>,
    mut zones: Query<(Entity, &mut PersistentZone)>,
    mut commands: Commands,
) {
    for (entity, mut zone) in &mut zones {
        zone.duration.tick(time.delta());
        if zone.duration.finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// TODO(Phase 6): implement.
pub fn move_anchored_zones(
    mut zones: Query<(&PersistentZone, &mut WorldPosition)>,
    positions: Query<&WorldPosition, Without<PersistentZone>>,
) {
    for (zone, mut zone_pos) in &mut zones {
        if let ZoneAnchor::Follow(followed) = zone.anchor {
            if let Ok(target_pos) = positions.get(followed) {
                zone_pos.0 = target_pos.0;
            }
        }
    }
}
