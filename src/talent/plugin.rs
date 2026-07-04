// TalentPlugin — wires the talent system into the app (Phase 2).
//
// Responsibilities:
//   - Registers TalentDef as a Bevy asset + its RON loader (*.talent.ron).
//   - Registers TalentLibrary (id → handle) and loads the talent RON files at startup.
//   - Attaches AcquiredTalents / ActiveHooks to the player on spawn.
//   - Registers TalentAcquiredEvent / TalentRemovedEvent and the install/uninstall systems.
//
// The install/uninstall systems run ungated by GameState: TalentAcquiredEvent is emitted from
// the TalentPicker state, so its reader must not be frozen with the InRun world.

use bevy::asset::AssetApp;
use bevy::prelude::*;
use crate::talent::assets::{TalentDef, TalentDefLoader, TalentLibrary};
use crate::talent::systems::apply::{
    attach_talent_components, install_acquired_talent, uninstall_removed_talent,
    TalentAcquiredEvent, TalentRemovedEvent,
};

pub struct TalentPlugin;

impl Plugin for TalentPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<TalentDef>()
            .register_asset_loader(TalentDefLoader)
            .init_resource::<TalentLibrary>()
            .add_event::<TalentAcquiredEvent>()
            .add_event::<TalentRemovedEvent>();

        app.add_systems(Startup, load_talent_defs);
        // attach runs in Update (not Startup) so `Added<Player>` reliably fires after the
        // Startup `spawn_player` — Startup system ordering relative to it is otherwise undefined.
        app.add_systems(
            Update,
            (attach_talent_components, install_acquired_talent, uninstall_removed_talent),
        );
    }
}

/// Loads each talent RON into the TalentLibrary, keyed by its id.
/// Phase 2 loads a fixed list (the implemented Death Strike talents + one placeholder Rare);
/// a later phase can scan the `talents/` folder as content grows.
fn load_talent_defs(asset_server: Res<AssetServer>, mut library: ResMut<TalentLibrary>) {
    const TALENTS: &[(&str, &str)] = &[
        ("death_strike_leech_common", "talents/death_strike_leech_common.talent.ron"),
        ("death_strike_range_common", "talents/death_strike_range_common.talent.ron"),
        ("death_strike_damage_common", "talents/death_strike_damage_common.talent.ron"),
        ("death_strike_bone_shield_epic", "talents/death_strike_bone_shield_epic.talent.ron"),
        ("blood_boil_dnd_range_rare", "talents/blood_boil_dnd_range_rare.talent.ron"),
    ];
    for (id, path) in TALENTS {
        library.defs.insert((*id).to_string(), asset_server.load(*path));
    }
}
