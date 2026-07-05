// Phase 6: Persistent ground zone system — D&D, Consecrated Ground, Tree Conduit, AMZ, etc.
// Live since Phase 6 (ZonePlugin in GameLogicPlugin).
//
// Central concept: instead of every system querying zone entities directly, a single
// `build_player_zone_presence` system rebuilds `PlayerZonePresence` each frame. Any
// system that gates behavior on zone presence reads this resource. Zone entities are
// only queried for lifetime/anchor management.
//
// Module map:
//   components.rs — PersistentZone, ZoneAnchor, PlayerZonePresence resource
//   systems/
//     lifetime.rs  — ticks zone timers (despawn expired) + follows the owner (FollowCaster)
//     presence.rs  — rebuilds PlayerZonePresence each frame
//     tick.rs      — occupant tick effects: DoT to opposing faction inside, regen the owner inside
//     block.rs     — AMZ: destroys opposing-faction projectiles entering a blocking zone

pub mod components;
pub mod plugin;
pub mod systems;
