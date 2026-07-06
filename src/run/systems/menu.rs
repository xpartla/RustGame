// Login + main-menu + character-select flow (Phase 7.5C boot flow; Login/Resume/Scoreboard added
// Phase 8).
//
// Logic-side (headless-testable): these read input and drive `GameState` / emit
// `StartRunRequest`/`ResumeRunRequest`; the screens (ui/screens/login.rs, main_menu.rs,
// character_select.rs, scoreboard.rs) only render. Keyboard-first (D4).
//
// Boot (D1, extended D4): the windowed game boots Login → Menu → CharacterSelect → run via
// `enter_login` (a `Startup` system, replacing Phase 7.5's `enter_main_menu`). The headless sim
// never runs it (it lives in `GamePlugin`, not `GameLogicPlugin`), so `Sim::new_arena` stays in
// InRun and the golden campaign is untouched. Every input system here lives in `GameLogicPlugin`
// (so it is sim-able) but is state-gated on a state the campaign never enters ⇒ inert there.

use bevy::app::AppExit;
use bevy::prelude::*;

use crate::core::def_library::DefAsset;
use crate::game::state::GameState;
use crate::hero::assets::HeroDef;
use crate::meta::state::{hero_is_unlocked, MetaState};
use crate::run::systems::persistence::ResumeRunRequest;
use crate::run::systems::reset::StartRunRequest;

/// Windowed-only boot (D4): send the app to the Login screen at Startup instead of auto-starting a
/// run. The transition applies before the first gated `Update`, so no InRun gameplay frame leaks.
pub fn enter_login(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Login);
}

/// Login input: any key advances to the main menu (single local profile, D4 — no credentials, no
/// multi-profile picker; see architecture-plan §6 Q3).
pub fn handle_login_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.get_just_pressed().next().is_some() {
        next_state.set(GameState::Menu);
    }
}

/// Main-menu input: Enter / 1 → New Run (CharacterSelect); 2 → Resume Run (only if a save exists);
/// 3 → Scoreboard; Esc → quit.
pub fn handle_main_menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    meta: Res<MetaState>,
    mut next_state: ResMut<NextState<GameState>>,
    mut resume: EventWriter<ResumeRunRequest>,
    mut exit: EventWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Digit1) {
        next_state.set(GameState::CharacterSelect);
    } else if keys.just_pressed(KeyCode::Digit2) {
        if meta.in_progress_run.is_some() {
            resume.write(ResumeRunRequest);
        }
    } else if keys.just_pressed(KeyCode::Digit3) {
        next_state.set(GameState::Scoreboard);
    } else if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

/// Scoreboard input: Esc returns to the main menu.
pub fn handle_scoreboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Menu);
    }
}

/// Character-select input: a digit picks the hero at that (1-based) `HeroDef::MANIFEST` index and
/// requests a run as that hero (through the shared `StartRunRequest` → reset path) — refused if the
/// hero is locked (Phase 8, §4; no hero is locked yet, D3, so this never rejects a pick today). Esc
/// → back to the menu.
pub fn handle_character_select_input(
    keys: Res<ButtonInput<KeyCode>>,
    meta: Res<MetaState>,
    mut requests: EventWriter<StartRunRequest>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Menu);
        return;
    }
    if let Some(i) = pressed_digit(&keys) {
        if let Some((id, _)) = HeroDef::MANIFEST.get(i) {
            if hero_is_unlocked(&meta, id) {
                requests.write(StartRunRequest { hero_id: id.to_string(), seed: rand::random::<u64>() });
            }
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
