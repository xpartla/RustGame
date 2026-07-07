// Standing-in-your-own-zone movement bonus (Phase 9.2 — AMZ's `amz_movespeed_rare` talent).
//
// `amz_movespeed_rare`'s effect is `Behavior("amz_movespeed")` — a flag checked directly here, not
// through HookRegistry (it's a standing character effect keyed on zone presence each frame, not a
// per-cast resolved param). Runs right after `build_player_zone_presence` (same
// MovementSet::Integrate chain, zone/plugin.rs) so it reacts to the freshest snapshot; that
// snapshot itself trails `apply_velocity` by one frame within THIS frame's Integrate, so the
// bonus is consumed by `apply_velocity` starting next frame — a one-frame lag consistent with
// every other zone-presence consumer (occupant tick effects, projectile blocking).

use bevy::prelude::*;
use crate::core::components::ZoneSpeedModifier;
use crate::player::components::Player;
use crate::talent::components::ActiveHooks;
use crate::zone::components::PlayerZonePresence;

/// 20% faster while standing inside your own AMZ with the talent active.
const AMZ_MOVESPEED_BONUS: f32 = 1.2;

pub fn resolve_zone_speed_bonus(
    mut commands: Commands,
    presence: Res<PlayerZonePresence>,
    mut players: Query<(Entity, &ActiveHooks, Option<&ZoneSpeedModifier>), With<Player>>,
) {
    for (entity, hooks, current) in &mut players {
        let boosted = hooks.contains("amz_movespeed") && presence.is_inside("amz");
        match (boosted, current) {
            (true, Some(m)) if m.0 == AMZ_MOVESPEED_BONUS => {}
            (true, _) => {
                commands.entity(entity).insert(ZoneSpeedModifier(AMZ_MOVESPEED_BONUS));
            }
            (false, Some(_)) => {
                commands.entity(entity).remove::<ZoneSpeedModifier>();
            }
            (false, None) => {}
        }
    }
}
