// Tree Conduit's Enhanced-attack consumer (Phase 9.4 — Druid). Mechanics: "within X range of the
// tree, your next animal attack is enhanced." Modeled as a per-frame top-up rather than an
// edge-triggered "just entered the zone" grant: while the player stands inside a "tree_conduit"
// zone AND is in Animal form AND currently holds zero Enhanced charges, top up to exactly one.
//
// This single rule also covers the Mechanics epic talent ("All animal attacks are enhanced while
// in tree range") without a second code path: because the top-up re-fires every frame the charge
// count is back at zero (i.e. immediately after Scratch/Ferocious Bite spends it), standing in
// range already keeps every subsequent Animal cast enhanced for free — the epic's "no per-attack
// limit" framing collapses into the same mechanic under this model, so it is deferred as a
// separate talent (see CHANGELOG "Phase 9.4").

use bevy::prelude::*;
use crate::hero::components::{ActiveStance, Charges};
use crate::player::components::Player;
use crate::zone::components::PlayerZonePresence;

pub fn tree_conduit_enhances_animal_attacks(
    zones: Res<PlayerZonePresence>,
    mut player: Query<(&ActiveStance, &mut Charges), With<Player>>,
) {
    let Ok((stance, mut charges)) = player.single_mut() else {
        return;
    };
    if stance.0 == "animal" && charges.current == 0 && zones.is_inside("tree_conduit") {
        charges.gain(1);
    }
}
