// UiPlugin — wires every screen's spawn/render/despawn systems into the app by state.
//
// The talent picker was the first real UI. Phase 7.5 adds the persistent in-run HUD plus the
// menu/pause/game-over/character-select/merchant overlays and the visual act-graph map view. Every
// screen is display-only (the `ui/` ground rule): it reads logic state and renders. Input for each
// lives in the owning logic domain (progression / run / talent / game), so the flows stay
// headless-testable and the UI itself never runs in the sim (it is registered under
// PresentationPlugin, which the sim never builds).
//
// Because the whole gameplay simulation is gated on GameState::InRun, entering any overlay state
// freezes the world behind it for free.

use bevy::prelude::*;
use crate::game::state::GameState;
use crate::ui::screens::character_select::{despawn_character_select, spawn_character_select};
use crate::ui::screens::game_over::{despawn_game_over, spawn_game_over};
use crate::ui::screens::hud;
use crate::ui::screens::login::{despawn_login, spawn_login};
use crate::ui::screens::main_menu::{despawn_main_menu, spawn_main_menu};
use crate::ui::screens::map_select::{despawn_map_select, spawn_map_select};
use crate::ui::screens::merchant::{despawn_merchant, render_merchant, spawn_merchant};
use crate::ui::screens::pause::{despawn_pause, spawn_pause};
use crate::ui::screens::scoreboard::{despawn_scoreboard, spawn_scoreboard};
use crate::ui::screens::talent_picker::{despawn_picker, render_talent_picker, spawn_picker_root};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // Talent picker (Phase 2).
        app.add_systems(OnEnter(GameState::TalentPicker), spawn_picker_root);
        app.add_systems(
            Update,
            render_talent_picker.run_if(in_state(GameState::TalentPicker)),
        );
        app.add_systems(OnExit(GameState::TalentPicker), despawn_picker);

        // Map-select branch picker (Phase 7). Phase 7.5D upgrades its presentation to a visual
        // act-graph view; the input contract (run/systems/select.rs) is unchanged.
        app.add_systems(OnEnter(GameState::MapSelect), spawn_map_select);
        app.add_systems(OnExit(GameState::MapSelect), despawn_map_select);

        // In-run HUD (Phase 7.5A) — spawned on entering a run, updated by change-detection. Its
        // whole registration lives in hud::plugin so every marker/system stays private to that module.
        hud::plugin(app);

        // Game-over screen (Phase 7.5B) — death / Act-3 victory.
        app.add_systems(OnEnter(GameState::GameOver), spawn_game_over);
        app.add_systems(OnExit(GameState::GameOver), despawn_game_over);

        // Pause screen (Phase 7.5B) — build inspector while frozen.
        app.add_systems(OnEnter(GameState::Paused), spawn_pause);
        app.add_systems(OnExit(GameState::Paused), despawn_pause);

        // Login (Phase 8, D4) + main menu + character select (Phase 7.5C).
        app.add_systems(OnEnter(GameState::Login), spawn_login);
        app.add_systems(OnExit(GameState::Login), despawn_login);
        app.add_systems(OnEnter(GameState::Menu), spawn_main_menu);
        app.add_systems(OnExit(GameState::Menu), despawn_main_menu);
        app.add_systems(OnEnter(GameState::CharacterSelect), spawn_character_select);
        app.add_systems(OnExit(GameState::CharacterSelect), despawn_character_select);

        // Scoreboard (Phase 8) — read-only run-history list, reached from the main menu.
        app.add_systems(OnEnter(GameState::Scoreboard), spawn_scoreboard);
        app.add_systems(OnExit(GameState::Scoreboard), despawn_scoreboard);

        // Merchant shop (Phase 7.5E) — re-rendered when the talent set changes (after a remove).
        app.add_systems(OnEnter(GameState::Merchant), spawn_merchant);
        app.add_systems(Update, render_merchant.run_if(in_state(GameState::Merchant)));
        app.add_systems(OnExit(GameState::Merchant), despawn_merchant);
    }
}
