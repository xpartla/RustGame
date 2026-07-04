// Phase 6: Persistent ground zone system — D&D, Consecrated Ground, Tree Conduit, etc.
//
// NOT yet wired into main.rs or GamePlugin. Add in Phase 6.
//
// Central concept: instead of every system querying zone entities directly, a single
// `build_player_zone_presence` system rebuilds `PlayerZonePresence` each frame. Any
// system that gates behavior on zone presence reads this resource. Zone entities are
// only queried for lifetime/anchor management.
//
// Module map:
//   components.rs — PersistentZone, ZoneAnchor, PlayerZonePresence resource
//   systems/
//     lifetime.rs  — ticks zone timers, despawns expired zones
//     presence.rs  — rebuilds PlayerZonePresence each frame

pub mod components;
pub mod plugin;
pub mod systems;
