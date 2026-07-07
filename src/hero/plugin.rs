// HeroPlugin — the hero / class-identity + stance system (Phase 4).
//
// Responsibilities:
//   - Registers HeroDef as a Bevy asset + `.hero.ron` loader + HeroLibrary + Startup populate
//     (via the generic register_def_library — see core/def_library.rs).
//   - resolve_input_to_ability: mouse input + ActiveStance → TriggerAbilityEvent (replaces the
//     Phase-1 stub player/systems/ability_input.rs).
//   - handle_stance_swap: Q → flip ActiveStance and apply the entered stance's swap effect.
//
// Both runtime systems run in InState(GameState::InRun), before CombatSet::Damage so the
// TriggerAbilityEvent reaches execute_ready_abilities (in that set) the same frame.

use bevy::prelude::*;
use crate::core::def_library::DefLibraryAppExt;
use crate::core::sets::CombatSet;
use crate::game::state::GameState;
use crate::hero::assets::HeroDef;
use crate::hero::systems::input_slot::resolve_input_to_ability;
use crate::hero::systems::resource::sync_charges_to_class_resource;
use crate::hero::systems::stance::handle_stance_swap;

pub struct HeroPlugin;

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.register_def_library::<HeroDef>();

        app.add_systems(
            Update,
            (handle_stance_swap, resolve_input_to_ability)
                .before(CombatSet::Damage)
                .run_if(in_state(GameState::InRun)),
        );
        // Class-resource bridge (Phase 9.1): mirrors Charges into the HUD's ClassResource whenever
        // content grants/spends them. No shipped hero carries Charges yet — inert.
        app.add_systems(
            Update,
            sync_charges_to_class_resource.run_if(in_state(GameState::InRun)),
        );

        // Debug-only: press M to become the Mage for manual playtesting (no character-select yet).
        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            crate::hero::systems::debug::debug_swap_to_mage.run_if(in_state(GameState::InRun)),
        );
    }
}
