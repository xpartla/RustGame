// Rebuilds PlayerZonePresence each frame.
//
// Sweeps all PersistentZone entities, tests player world-distance against zone radius,
// and writes the result into the PlayerZonePresence resource.
//
// Runs at the start of Update (before ability execution) so all ability/talent systems
// that frame see a fresh presence snapshot.
//
// This is the only system that queries zone entities for spatial testing. All other systems
// read PlayerZonePresence instead.

use bevy::prelude::*;
use crate::core::components::WorldPosition;
use crate::player::components::Player;
use crate::zone::components::{PersistentZone, PlayerZonePresence, ZoneAnchor};

/// TODO(Phase 6): implement.
pub fn build_player_zone_presence(
    player_q: Query<&WorldPosition, With<Player>>,
    zones: Query<(&PersistentZone, &WorldPosition)>,
    mut presence: ResMut<PlayerZonePresence>,
) {
    presence.active_zone_types.clear();

    let Ok(player_pos) = player_q.single() else {
        return;
    };

    for (zone, zone_pos) in &zones {
        let center = match zone.anchor {
            ZoneAnchor::Fixed(v) => v,
            ZoneAnchor::Follow(_) => zone_pos.0, // already updated by move_anchored_zones
        };
        if player_pos.0.distance(center) <= zone.radius {
            presence.active_zone_types.insert(zone.zone_type.clone());
        }
    }
}
