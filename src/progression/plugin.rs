// ProgressionPlugin — the leveling & talent-offer flow (Phase 2).
//
// Responsibilities:
//   - Inserts LevelUpFlowState at startup (band pools shuffled with RunRng).
//   - Registers ThroneRoomRewardEvent (consumer lands in Phase 7).
//   - handle_level_up: consumes LevelUpEvent after gain_experience, drives phase transitions,
//     emits UnlockAbilityEvent, and enters the TalentPicker overlay when a choice is owed.
//   - refill_offer + handle_talent_choice: drain the owed-choice backlog through the overlay.
//   - debug_force_level_up (dev builds): `L` to fast-forward a level.

use bevy::prelude::*;
use crate::game::state::GameState;
use crate::player::systems::experience::gain_experience;
use crate::progression::systems::level_up::{handle_level_up, init_level_flow};
use crate::progression::systems::offer::{
    handle_talent_choice, handle_throne_room_reward, refill_offer, ThroneRoomRewardEvent,
};
use crate::talent::systems::apply::install_acquired_talent;
use crate::world::systems::generate_map::generate_map;

pub struct ProgressionPlugin;

impl Plugin for ProgressionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ThroneRoomRewardEvent>();

        // Ordered after generate_map: both Startup systems draw from RunRng, and without an
        // explicit constraint the executor may run them in either order — meaning the same
        // seed could produce different maps and band-shuffle orders between launches.
        app.add_systems(Startup, init_level_flow.after(generate_map));

        app.add_systems(
            Update,
            handle_level_up
                .after(gain_experience)
                .run_if(in_state(GameState::InRun)),
        );

        // ThroneRoom reward (Phase 7F): consumes ThroneRoomRewardEvent (emitted by load_encounter)
        // and opens the Rare-floor picker. InRun-gated so it reads the event before the world freezes.
        app.add_systems(
            Update,
            handle_throne_room_reward.run_if(in_state(GameState::InRun)),
        );

        // Ordered after install_acquired_talent: when the backlog holds several owed choices,
        // the next offer must be generated *after* the previous pick landed in AcquiredTalents,
        // or uniqueness filtering (Stack/Exclusive) could sample against stale state and offer
        // a just-taken talent again.
        app.add_systems(
            Update,
            (refill_offer, handle_talent_choice)
                .chain()
                .after(install_acquired_talent)
                .run_if(in_state(GameState::TalentPicker)),
        );

        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            crate::progression::systems::level_up::debug_force_level_up
                .run_if(in_state(GameState::InRun)),
        );
    }
}
