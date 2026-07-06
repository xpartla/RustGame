// Login screen (Phase 8, D4) — the windowed boot splash (`GameState::Login`), shown before the
// main menu. Local-profile only (architecture-plan §6 Q3: no credentials, no multi-profile picker,
// no networked/cloud save) — this screen exists purely so the boot flow has a "Log In" beat before
// Menu, per Mechanics.md's user-flow sketch. Display only; input is logic-side
// (run/systems/menu.rs::handle_login_input, any key advances to Menu). Never runs headless.

use bevy::prelude::*;
use crate::ui::theme::{self, text};

#[derive(Component)]
pub struct LoginRoot;

pub fn spawn_login(mut commands: Commands) {
    commands
        .spawn((LoginRoot, theme::overlay_root(), BackgroundColor(theme::OVERLAY_BG)))
        .with_children(|root| {
            root.spawn(text("RUSTGAME", theme::FS_TITLE, theme::ACCENT));
            root.spawn(text("Local profile", theme::FS_BODY, theme::TEXT));
            root.spawn(text("Press any key to continue", theme::FS_HINT, theme::HINT));
        });
}

pub fn despawn_login(mut commands: Commands, root: Query<Entity, With<LoginRoot>>) {
    for e in &root {
        commands.entity(e).despawn();
    }
}
