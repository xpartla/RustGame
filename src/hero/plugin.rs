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
use crate::hero::systems::enhanced::tree_conduit_enhances_animal_attacks;
use crate::hero::systems::input_slot::resolve_input_to_ability;
use crate::hero::systems::resource::sync_charges_to_class_resource;
use crate::hero::systems::stance::handle_stance_swap;

pub struct HeroPlugin;

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.register_def_library::<HeroDef>();

        app.add_systems(
            Update,
            (handle_stance_swap, resolve_input_to_ability, tree_conduit_enhances_animal_attacks)
                .before(CombatSet::Damage)
                .run_if(in_state(GameState::InRun)),
        );
        // Class-resource bridge (Phase 9.1): mirrors Charges into the HUD's ClassResource whenever
        // content grants/spends them. Pinned `.after(CombatSet::Death)` (originally just
        // `.after(CombatSet::Damage)`, Phase 9.4 — found once the Druid became the first real
        // `Charges` consumer; strengthened Phase 9.5, when the Mage's frost-charge-on-frostbitten-
        // kill talent became the first mutator living in CombatSet::Death instead of Damage, and
        // hit the identical one-frame-stale gap the Phase 9.4 fix was meant to close — "after
        // Damage" doesn't imply "after Death," since Death merely comes later in the SAME chain,
        // not before this unordered system). Every current mutator (`execute_ready_abilities`'s
        // Scratch/Ferocious Bite spend, `tick_channels`'s Heal/Frost-Impale grant+spend,
        // `collect_pickups`'s Bloom grant, `ability::systems::mage_frost_kill`'s kill-reactive
        // grant) now lives at or before CombatSet::Death, so this is the one pin that covers all of
        // them without needing to grow every time a new mutator's home set changes.
        app.add_systems(
            Update,
            sync_charges_to_class_resource
                .after(CombatSet::Death)
                .run_if(in_state(GameState::InRun)),
        );

        // Debug-only: press M to become the Mage for manual playtesting (no character-select yet).
        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            crate::hero::systems::debug::debug_swap_to_mage.run_if(in_state(GameState::InRun)),
        );
    }
}
