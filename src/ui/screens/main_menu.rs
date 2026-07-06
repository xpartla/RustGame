// Main menu (Phase 7.5C) — the windowed boot screen (`GameState::Menu`).
//
// Display only; input is logic-side (run/systems/menu.rs::handle_main_menu_input). Resume Run is
// greyed unless `MetaState.in_progress_run` holds a save (Phase 8); Scoreboard is always live.
// Never runs headless.

use bevy::prelude::*;
use crate::meta::state::MetaState;
use crate::ui::theme::{self, text};

#[derive(Component)]
pub struct MainMenuRoot;

pub fn spawn_main_menu(mut commands: Commands, meta: Res<MetaState>) {
    let (resume_label, resume_color) = if meta.in_progress_run.is_some() {
        ("2.   Resume Run", theme::TEXT)
    } else {
        ("2.   Resume Run    (no saved run)", theme::DIM)
    };

    commands
        .spawn((MainMenuRoot, theme::overlay_root(), BackgroundColor(theme::OVERLAY_BG)))
        .with_children(|root| {
            root.spawn(text("RUSTGAME", theme::FS_TITLE, theme::ACCENT));
            root.spawn(text("1.   New Run", theme::FS_BODY, theme::TEXT));
            root.spawn(text(resume_label, theme::FS_BODY, resume_color));
            root.spawn(text("3.   Scoreboard", theme::FS_BODY, theme::TEXT));
            root.spawn(text("Enter — New Run       ·       Esc — Quit", theme::FS_HINT, theme::HINT));
        });
}

pub fn despawn_main_menu(mut commands: Commands, root: Query<Entity, With<MainMenuRoot>>) {
    for e in &root {
        commands.entity(e).despawn();
    }
}
