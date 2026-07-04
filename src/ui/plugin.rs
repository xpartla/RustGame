// UiPlugin — wires the (minimal, Phase 2) UI screens into the app.
//
// The talent picker is the first real UI. It spawns a full-screen overlay on entering
// GameState::TalentPicker, re-renders its option rows whenever the pending offer changes, and
// tears the overlay down on exit. Because the whole gameplay simulation is gated on
// GameState::InRun, the world is frozen behind the overlay for free.

use bevy::prelude::*;
use crate::game::state::GameState;
use crate::ui::screens::talent_picker::{despawn_picker, render_talent_picker, spawn_picker_root};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::TalentPicker), spawn_picker_root);
        app.add_systems(
            Update,
            render_talent_picker.run_if(in_state(GameState::TalentPicker)),
        );
        app.add_systems(OnExit(GameState::TalentPicker), despawn_picker);
    }
}
