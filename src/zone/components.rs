// Persistent ground zone types and the per-frame presence cache.
//
// Zone types (as ZoneTypeId strings, defined in ability RON files):
//   "death_and_decay"       — DK D&D: buffs Death Strike damage, buffs Heart Strike, heals DK
//   "consecrated_ground"    — Paladin: damages enemies, enables Hammer/Flash of Light combos
//   "tree_conduit"          — Druid: enhances animal attacks within range
//   "amz"                   — DK: blocks projectiles; epic variant follows the player
//   "bloom_flower"          — Druid: pickup-on-run-over, grants enhanced charge
//
// ZoneTypeId is a plain String (matches the zone_type field in the ability's base_params).
// Adding a new zone: choose a unique string, use it in the ability's RON and in the
// talent/behavior that checks for it via PlayerZonePresence.
//
// Interactions:
//   - zone/systems/presence.rs builds PlayerZonePresence from all PersistentZone entities.
//   - ability/behavior.rs uses PlayerZonePresence in zone-conditional behaviors.
//   - talent hooks read PlayerZonePresence (e.g. Blood Boil double range inside D&D).

use bevy::prelude::*;
use std::collections::HashSet;

pub type ZoneTypeId = String;

/// A persistent ground zone in the world. Entity is a child of its owner.
#[derive(Component, Debug)]
pub struct PersistentZone {
    pub zone_type: ZoneTypeId,
    /// The entity that created this zone (player or summon).
    pub owner: Entity,
    pub radius: f32,
    pub duration: Timer,
    pub anchor: ZoneAnchor,
}

/// Where the zone's center is located.
#[derive(Debug, Clone)]
pub enum ZoneAnchor {
    /// Spawned at a fixed world position (most zones).
    Fixed(bevy::math::Vec2),
    /// Follows an entity (AMZ epic talent — zone follows the player).
    Follow(Entity),
}

/// Resource rebuilt every frame by zone/systems/presence.rs.
/// Systems that gate on zone presence read this, not the zone entities directly.
///
/// Example usage in a talent hook:
///   if presence.is_inside("death_and_decay") { apply_damage_bonus(); }
#[derive(Resource, Default, Debug)]
pub struct PlayerZonePresence {
    pub active_zone_types: HashSet<ZoneTypeId>,
}

impl PlayerZonePresence {
    pub fn is_inside(&self, zone_type: &str) -> bool {
        self.active_zone_types.contains(zone_type)
    }
}
