// Game-over screen (Phase 7.5B) — shown in `GameState::GameOver` on death or an Act-3 clear.
//
// Display only. Renders the `GameOverSummary` snapshot (the run's entities/resources are already
// gone); input (R restart / M main menu) is handled logic-side by
// run/systems/reset.rs::handle_game_over_input. Never runs headless.

use bevy::prelude::*;

use crate::game::state::GameOverSummary;
use crate::ui::theme::{self, text};

#[derive(Component)]
pub struct GameOverRoot;

pub fn spawn_game_over(mut commands: Commands, summary: Option<Res<GameOverSummary>>) {
    let (title, title_color, lines) = match summary.as_deref() {
        Some(s) if s.victory => (
            "VICTORY".to_string(),
            theme::ACCENT,
            summary_lines(s),
        ),
        Some(s) => ("YOU DIED".to_string(), theme::DANGER, summary_lines(s)),
        None => ("GAME OVER".to_string(), theme::DANGER, Vec::new()),
    };

    commands
        .spawn((GameOverRoot, theme::overlay_root(), BackgroundColor(theme::OVERLAY_BG)))
        .with_children(|root| {
            root.spawn(text(title, theme::FS_TITLE, title_color));
            for line in lines {
                root.spawn(text(line, theme::FS_BODY, theme::TEXT));
            }
            root.spawn(text("R — Restart      ·      M — Main Menu", theme::FS_HINT, theme::HINT));
        });
}

fn summary_lines(s: &GameOverSummary) -> Vec<String> {
    let mut lines = vec![format!("Hero: {}", s.hero_id), format!("Level {}", s.level)];
    if let Some(act) = s.act {
        // `node_column` is 0-based; show it 1-based to match the HUD's "Node N/15".
        let node = s.node_column.map(|c| c + 1).unwrap_or(1);
        lines.push(format!("Reached Act {} · Node {}", act, node));
    }
    lines
}

pub fn despawn_game_over(mut commands: Commands, root: Query<Entity, With<GameOverRoot>>) {
    for e in &root {
        commands.entity(e).despawn();
    }
}
