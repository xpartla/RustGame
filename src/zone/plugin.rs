// ZonePlugin — brings the persistent-zone system online (Phase 6).
//
// Responsibilities:
//   - Inserts the PlayerZonePresence resource (the per-frame spatial cache).
//   - Runs zone maintenance at the END of MovementSet::Integrate (after apply_velocity →
//     world_to_grid), so positions are already settled for this frame and the presence snapshot
//     is fresh BEFORE CombatSet::Damage, where ability execution + zone-tick effects read it:
//       tick_zone_lifetimes    — reap expired zones (so a dead zone can't grant presence)
//       move_anchored_zones    — Follow-anchored zones catch up to their owner
//       build_player_zone_presence — rebuild PlayerZonePresence from the surviving zones
//       resolve_zone_speed_bonus  — AMZ's move-speed talent (Phase 9.2), reacts to that snapshot
//   - (6D/6E add zone_tick_effects + block_projectiles_in_zones in CombatSet::Damage.)
//
// All systems run in InState(GameState::InRun). Placing maintenance after `world_to_grid`
// (not before combat generically) keeps the Phase-3.1 movement pin intact: zone systems never
// write an actor's WorldPosition, so with zero zones alive they are empty-loop no-ops and the
// golden master is unaffected.

use bevy::prelude::*;
use crate::core::sets::{CombatSet, MovementSet};
use crate::core::systems::grid_sync::world_to_grid;
use crate::game::state::GameState;
use crate::projectile::systems::motion::{move_projectiles, projectile_collision};
use crate::zone::components::PlayerZonePresence;
use crate::zone::systems::block::block_projectiles_in_zones;
use crate::zone::systems::lifetime::{move_anchored_zones, tick_zone_lifetimes};
use crate::zone::systems::presence::build_player_zone_presence;
use crate::zone::systems::speed_bonus::resolve_zone_speed_bonus;
use crate::zone::systems::tick::zone_tick_effects;

pub struct ZonePlugin;

impl Plugin for ZonePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerZonePresence>();
        app.add_systems(
            Update,
            (
                // Zone maintenance at the end of MovementSet::Integrate: reap expired, follow the
                // owner, then rebuild the presence snapshot — all before CombatSet reads it.
                (tick_zone_lifetimes, move_anchored_zones, build_player_zone_presence, resolve_zone_speed_bonus)
                    .chain()
                    .in_set(MovementSet::Integrate)
                    .after(world_to_grid),
                // Occupant tick effects (Phase 6D) emit DamageEvent/HealEvent, so they belong in
                // CombatSet::Damage alongside the other emitters.
                zone_tick_effects.in_set(CombatSet::Damage),
                // AMZ projectile blocking (Phase 6E): after the shot moves, before it can collide.
                block_projectiles_in_zones
                    .in_set(CombatSet::Damage)
                    .after(move_projectiles)
                    .before(projectile_collision),
            )
                .run_if(in_state(GameState::InRun)),
        );
    }
}
