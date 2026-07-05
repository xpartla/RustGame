// Main menu (Phase 7.5C) — the windowed boot screen (`GameState::Menu`).
//
// Display only; input is logic-side (run/systems/menu.rs::handle_main_menu_input). Resume Run and
// Scoreboard are shown greyed — they need Phase-8 persistence (RunState serialization / a score
// formula), so they are deliberately inert this phase. Never runs headless.

use bevy::prelude::*;
use crate::ui::theme::{self, text};

#[derive(Component)]
pub struct MainMenuRoot;

pub fn spawn_main_menu(mut commands: Commands) {
    commands
        .spawn((MainMenuRoot, theme::overlay_root(), BackgroundColor(theme::OVERLAY_BG)))
        .with_children(|root| {
            root.spawn(text("RUSTGAME", theme::FS_TITLE, theme::ACCENT));
            root.spawn(text("1.   New Run", theme::FS_BODY, theme::TEXT));
            root.spawn(text("Resume Run    (locked — Phase 8)", theme::FS_BODY, theme::DIM));
            root.spawn(text("Scoreboard    (locked — Phase 8)", theme::FS_BODY, theme::DIM));
            root.spawn(text("Enter — New Run       ·       Esc — Quit", theme::FS_HINT, theme::HINT));
        });
}

pub fn despawn_main_menu(mut commands: Commands, root: Query<Entity, With<MainMenuRoot>>) {
    for e in &root {
        commands.entity(e).despawn();
    }
}
