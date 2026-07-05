// Pause screen (Phase 7.5B) — shown in `GameState::Paused` (Esc from InRun).
//
// Display only. Doubles as the playtester's build inspector: it lists the player's unlocked abilities
// and acquired talents (with stack counts) — a snapshot read on entry, since the world is frozen
// behind the overlay. The Esc toggle back to InRun is handled logic-side (game/pause.rs). Never runs
// headless.

use bevy::prelude::*;

use crate::ability::components::AbilityInstance;
use crate::player::components::Player;
use crate::talent::assets::{TalentDef, TalentLibrary};
use crate::talent::components::AcquiredTalents;
use crate::ui::theme::{self, text};

#[derive(Component)]
pub struct PauseRoot;

pub fn spawn_pause(
    mut commands: Commands,
    players: Query<Entity, With<Player>>,
    acquired: Query<&AcquiredTalents, With<Player>>,
    instances: Query<&AbilityInstance>,
    library: Res<TalentLibrary>,
    defs: Res<Assets<TalentDef>>,
) {
    let player = players.single().ok();

    // Unlocked abilities (owned instances).
    let mut abilities: Vec<String> = instances
        .iter()
        .filter(|i| Some(i.owner) == player)
        .map(|i| i.def_id.clone())
        .collect();
    abilities.sort();
    abilities.dedup();

    // Acquired talents (name [rarity] ×count).
    let talents: Vec<(String, Option<crate::talent::assets::TalentRarity>)> = acquired
        .single()
        .map(|a| {
            a.entries
                .iter()
                .map(|(id, count)| {
                    let def = library.get(id).and_then(|h| defs.get(h));
                    let name = def.map(|d| d.display_name.clone()).unwrap_or_else(|| id.clone());
                    let rarity = def.map(|d| d.rarity.clone());
                    (if *count > 1 { format!("{name}  ×{count}") } else { name }, rarity)
                })
                .collect()
        })
        .unwrap_or_default();

    commands
        .spawn((PauseRoot, theme::overlay_root(), BackgroundColor(theme::OVERLAY_BG)))
        .with_children(|root| {
            root.spawn(text("PAUSED", theme::FS_TITLE, theme::TITLE));

            root.spawn(text("Abilities", theme::FS_HEADING, theme::ACCENT));
            if abilities.is_empty() {
                root.spawn(text("(none)", theme::FS_SMALL, theme::DIM));
            }
            for a in &abilities {
                root.spawn(text(a.to_uppercase(), theme::FS_SMALL, theme::TEXT));
            }

            root.spawn(text("Talents", theme::FS_HEADING, theme::ACCENT));
            if talents.is_empty() {
                root.spawn(text("(none)", theme::FS_SMALL, theme::DIM));
            }
            for (label, rarity) in &talents {
                let color = rarity.as_ref().map(theme::rarity_color).unwrap_or(theme::TEXT);
                root.spawn(text(label.clone(), theme::FS_SMALL, color));
            }

            root.spawn(text("Esc — Resume", theme::FS_HINT, theme::HINT));
        });
}

pub fn despawn_pause(mut commands: Commands, root: Query<Entity, With<PauseRoot>>) {
    for e in &root {
        commands.entity(e).despawn();
    }
}
