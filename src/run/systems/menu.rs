// Main-menu + character-select flow (Phase 7.5C).
//
// Logic-side (headless-testable): these read input and drive `GameState` / emit `StartRunRequest`;
// the screens (ui/screens/main_menu.rs, character_select.rs) only render. Keyboard-first (D4).
//
// Boot (D1): the windowed game replaces Phase 7's `auto_start_run` with `enter_main_menu`, so it
// boots Menu → CharacterSelect → run. The headless sim never runs `enter_main_menu` (it lives in
// GamePlugin, not GameLogicPlugin), so `Sim::new_arena` stays in InRun and the golden campaign is
// untouched. The menu/character-select *input* systems live in GameLogicPlugin (so they are sim-able)
// but are state-gated on Menu / CharacterSelect, which the campaign never enters ⇒ inert there.

use bevy::app::AppExit;
use bevy::prelude::*;

use crate::core::def_library::DefAsset;
use crate::game::state::GameState;
use crate::hero::assets::HeroDef;
use crate::run::systems::reset::StartRunRequest;

/// Windowed-only boot (D1): send the app to the main menu at Startup instead of auto-starting a run.
/// The transition applies before the first gated `Update`, so no InRun gameplay frame leaks.
pub fn enter_main_menu(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Menu);
}

/// Main-menu input: Enter / 1 → New Run (CharacterSelect); Esc → quit. Resume Run + Scoreboard are
/// shown greyed and do nothing (Phase 8).
pub fn handle_main_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: EventWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Digit1) {
        next_state.set(GameState::CharacterSelect);
    } else if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

/// Character-select input: a digit picks the hero at that (1-based) `HeroDef::MANIFEST` index and
/// requests a run as that hero (through the shared `StartRunRequest` → reset path); Esc → back to the
/// menu. All heroes are selectable until Phase-8 `MetaState` unlock persistence exists.
pub fn handle_character_select_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut requests: EventWriter<StartRunRequest>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Menu);
        return;
    }
    if let Some(i) = pressed_digit(&keys) {
        if let Some((id, _)) = HeroDef::MANIFEST.get(i) {
            requests.write(StartRunRequest { hero_id: id.to_string(), seed: rand::random::<u64>() });
        }
    }
}

/// Maps a just-pressed digit key to a 0-based index (1→0 … 4→3).
fn pressed_digit(keys: &ButtonInput<KeyCode>) -> Option<usize> {
    if keys.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(3)
    } else {
        None
    }
}
