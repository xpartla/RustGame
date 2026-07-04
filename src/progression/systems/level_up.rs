// Consumes LevelUpEvent and drives the two-phase progression flow, plus run-start flow setup
// and a debug force-level key.
//
// On each LevelUpEvent (runs after gain_experience, which runs after CombatSet::Death):
//   AbilityUnlock phase (L2–L6): draw the next band ability and emit UnlockAbilityEvent.
//     The band ability's behavior may not exist yet in Phase 2 — the ability plugin spawns an
//     inert AbilityInstance and execution skips it gracefully.
//   TalentChoices phase (L7+): owe the player a talent choice and enter the TalentPicker overlay.
//     Offer generation itself is lazy (progression/systems/offer.rs::refill_offer) so uniqueness
//     reflects each acquisition as the backlog drains.
//
// Level-banding is intrinsic to the pool order (2/3 pool first, then 4/6); see
// LevelUpFlowState::next_unlock.

use bevy::prelude::*;
use rand::seq::SliceRandom;
use crate::ability::assets::AbilityId;
use crate::ability::components::UnlockAbilityEvent;
use crate::core::events::LevelUpEvent;
use crate::game::state::GameState;
use crate::player::components::Player;
use crate::progression::state::{LevelUpFlowState, LevelUpPhase};
use crate::run::rng::RunRng;

// PHASE 2 STUB: hardcoded Blood Death Knight band pools. Phase 4 sources these from
// HeroDef.band_2_3_pool / band_4_6_pool once the hero asset pipeline is wired.
const BDK_BAND_2_3: &[&str] = &["blood_boil", "heart_strike"];
const BDK_BAND_4_6: &[&str] = &["abomination_limb", "purgatory", "amz"];

/// Inserts the LevelUpFlowState resource at startup, shuffling the band pools with RunRng so
/// the draw order is seed-deterministic. Phase 7 moves this into the run-start flow.
pub fn init_level_flow(mut commands: Commands, mut rng: ResMut<RunRng>) {
    let mut band_2_3: Vec<AbilityId> = BDK_BAND_2_3.iter().map(|s| s.to_string()).collect();
    let mut band_4_6: Vec<AbilityId> = BDK_BAND_4_6.iter().map(|s| s.to_string()).collect();
    band_2_3.shuffle(rng.rng());
    band_4_6.shuffle(rng.rng());
    commands.insert_resource(LevelUpFlowState::new(band_2_3, band_4_6));
}

/// Advances the level-up flow for each LevelUpEvent received this frame.
pub fn handle_level_up(
    mut level_events: EventReader<LevelUpEvent>,
    mut flow: ResMut<LevelUpFlowState>,
    mut unlocks: EventWriter<UnlockAbilityEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    players: Query<Entity, With<Player>>,
) {
    let Ok(player) = players.single() else {
        return; // no player (e.g. dead) — leave events for a later frame
    };

    let mut owe_picker = false;
    for _ev in level_events.read() {
        match flow.phase {
            LevelUpPhase::AbilityUnlock => {
                if let Some(id) = flow.next_unlock() {
                    unlocks.write(UnlockAbilityEvent { ability_id: id, owner: player });
                }
            }
            LevelUpPhase::TalentChoices => {
                flow.record_talent_level();
                owe_picker = true;
            }
        }
    }

    if owe_picker {
        next_state.set(GameState::TalentPicker);
    }
}

/// DEBUG (dev builds only): press `L` to grant exactly enough XP to reach the next level,
/// so the talent flow is easy to exercise without grinding kills. Flows through the normal
/// GainXpEvent → gain_experience → LevelUpEvent path.
#[cfg(debug_assertions)]
pub fn debug_force_level_up(
    keys: Res<ButtonInput<KeyCode>>,
    mut xp_events: EventWriter<crate::core::events::GainXpEvent>,
    players: Query<(Entity, &crate::player::components::Experience), With<Player>>,
) {
    if !keys.just_pressed(KeyCode::KeyL) {
        return;
    }
    let Ok((player, exp)) = players.single() else {
        return;
    };
    let needed = exp.to_next.saturating_sub(exp.current).max(1);
    xp_events.write(crate::core::events::GainXpEvent { target: player, amount: needed });
    info!("[debug] forcing level-up: +{needed} xp");
}
