// Pause toggle (Phase 7.5B) — Esc flips between active play and the pause overlay.
//
// Logic-side (so it is headless-testable): reads only state and writes `NextState`. Gameplay is
// already frozen in `Paused` by the blanket `in_state(InRun)` gating, and in-flight combat events
// survive the freeze (the `add_gameplay_event` contract, docs/testing.md / freeze.rs).
//
// Registered with an `input_just_pressed(Escape)` run condition, so it only runs on a frame where
// Esc is actually pressed. The golden campaign never presses Esc ⇒ the system never runs there ⇒
// byte-identical. Esc means "decline" in the TalentPicker and is unused in MapSelect/Merchant; those
// states are ignored here (the match only toggles InRun ⇄ Paused), so there is no cross-state
// conflict.

use bevy::prelude::*;
use crate::game::state::GameState;

pub fn toggle_pause(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    match state.get() {
        GameState::InRun => next_state.set(GameState::Paused),
        GameState::Paused => next_state.set(GameState::InRun),
        _ => {}
    }
}
