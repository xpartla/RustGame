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

/// Per-tick occupant effects a zone applies (Phase 6D). Present only on zones whose ability defines
/// any (Consecrated Ground DoT, D&D regen); a pure marker zone (Tree Conduit) carries none. Baked
/// from resolved params at spawn; `zone/systems/tick.rs::zone_tick_effects` reads it.
#[derive(Component, Debug)]
pub struct ZoneEffects {
    /// Damage dealt to each opposing-faction actor inside, per tick (0 = buff/marker zone).
    pub damage_per_second: f32,
    /// Fraction of the owner's max health healed per tick while the owner stands inside (0 = none).
    pub regen_fraction: f32,
    /// Fixed-cadence tick timer (ZONE_TICK_INTERVAL, repeating). Discrete ticks keep it
    /// deterministic (no per-frame f32 accumulation).
    pub tick: Timer,
    /// A status effect (by id) applied to every damaged occupant each tick (Phase 9.3 — Consecrated
    /// Ground's `consecrated_ground_slow_common` talent). `None` for every other zone.
    pub slow_status: Option<crate::status::assets::StatusEffectId>,
    /// "Deals X% increased damage per enemy inside" (Phase 9.3 — Consecrated Ground's
    /// `consecrated_ground_count_scaling_rare` talent). `false` for every other zone.
    pub scales_with_occupants: bool,
}

/// Marker: this zone destroys opposing-faction projectiles that enter it (AMZ, Phase 6E). Present
/// only on zones whose ability sets `blocks_projectiles: true`.
#[derive(Component, Debug)]
pub struct ZoneBlocksProjectiles;

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
