// Scoreboard screen (Phase 8) — read-only run-history list (`GameState::Scoreboard`), reached from
// the main menu. Display only; the score math itself is the pure `meta::score::compute_score`
// (already applied when each `RunRecord` was appended — this screen just renders + sorts). Input is
// logic-side (run/systems/menu.rs::handle_scoreboard_input, Esc → Menu). Never runs headless.

use bevy::prelude::*;

use crate::meta::state::MetaState;
use crate::ui::theme::{self, text};

#[derive(Component)]
pub struct ScoreboardRoot;

pub fn spawn_scoreboard(mut commands: Commands, meta: Res<MetaState>) {
    let mut history = meta.run_history.clone();
    history.sort_by(|a, b| b.score.cmp(&a.score));

    commands
        .spawn((ScoreboardRoot, theme::overlay_root(), BackgroundColor(theme::OVERLAY_BG)))
        .with_children(|root| {
            root.spawn(text("SCOREBOARD", theme::FS_TITLE, theme::TITLE));
            if history.is_empty() {
                root.spawn(text("No runs recorded yet.", theme::FS_BODY, theme::DIM));
            }
            for (i, record) in history.iter().take(10).enumerate() {
                root.spawn(text(
                    format!(
                        "{}.   {}   —   Act {}   —   {} pts",
                        i + 1,
                        record.hero_id,
                        record.act_reached,
                        record.score
                    ),
                    theme::FS_BODY,
                    theme::TEXT,
                ));
            }
            root.spawn(text("Esc — Back", theme::FS_HINT, theme::HINT));
        });
}

pub fn despawn_scoreboard(mut commands: Commands, root: Query<Entity, With<ScoreboardRoot>>) {
    for e in &root {
        commands.entity(e).despawn();
    }
}
