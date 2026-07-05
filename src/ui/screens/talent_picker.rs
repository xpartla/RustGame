// Talent picker overlay — the level-up "choose 1 of 3" screen (GameState::TalentPicker).
//
// Display only. It reads LevelUpFlowState.pending_offer (populated by
// progression/systems/offer.rs::refill_offer) and renders one row per option, resolving each
// talent id to its display name + rarity via TalentLibrary. Player input (1/2/3, Esc) is handled
// by progression/systems/offer.rs::handle_talent_choice, which mutates the flow state; this
// screen re-renders whenever that state changes.
//
// Structure: a full-screen root holds a static title, an OptionsContainer (whose children are
// rebuilt on each offer change), and a static footer hint. Chrome + rarity colors come from
// ui/theme.rs (Phase 7.5A) so it matches every other screen.

use bevy::prelude::*;
use crate::progression::state::LevelUpFlowState;
use crate::talent::assets::{TalentDef, TalentLibrary};
use crate::ui::theme::{self, text};

/// Root overlay node. Despawned on exit (recursively removes the whole subtree).
#[derive(Component)]
pub struct TalentPickerRoot;

/// The node whose children are the per-option rows; rebuilt each time the offer changes.
#[derive(Component)]
pub struct OptionsContainer;

/// Marks a single option row so a re-render can clear the previous rows.
#[derive(Component)]
pub struct OptionRow;

/// Spawns the static overlay chrome on entering the TalentPicker state.
pub fn spawn_picker_root(mut commands: Commands) {
    commands
        .spawn((TalentPickerRoot, theme::overlay_root(), BackgroundColor(theme::OVERLAY_BG)))
        .with_children(|root| {
            root.spawn(text("LEVEL UP  —  Choose a Talent", theme::FS_TITLE, theme::ACCENT));
            root.spawn((
                OptionsContainer,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
            ));
            root.spawn(text("1 / 2 / 3 to choose      ·      Esc to skip", theme::FS_HINT, theme::HINT));
        });
}

/// Rebuilds the option rows whenever the flow state changes (offer generated / replaced).
pub fn render_talent_picker(
    mut commands: Commands,
    flow: Res<LevelUpFlowState>,
    container: Query<Entity, With<OptionsContainer>>,
    rows: Query<Entity, With<OptionRow>>,
    library: Res<TalentLibrary>,
    defs: Res<Assets<TalentDef>>,
) {
    if !flow.is_changed() {
        return;
    }
    let Ok(container) = container.single() else {
        return; // overlay not spawned yet this frame
    };

    // Clear previous rows.
    for row in &rows {
        commands.entity(row).despawn();
    }

    let Some(offer) = &flow.pending_offer else {
        return;
    };

    commands.entity(container).with_children(|parent| {
        if offer.options.is_empty() {
            parent.spawn((
                OptionRow,
                text("(no eligible talents — press Esc to skip)", theme::FS_BODY, theme::DIM),
            ));
            return;
        }
        for (i, id) in offer.options.iter().enumerate() {
            let resolved = library.get(id).and_then(|h| defs.get(h));
            let color = resolved.map(|d| theme::rarity_color(&d.rarity)).unwrap_or(theme::TEXT);
            let label = match resolved {
                Some(d) => format!("{}.   {}   [{:?}]", i + 1, d.display_name, d.rarity),
                None => format!("{}.   {}   [?]", i + 1, id),
            };
            parent.spawn((OptionRow, text(label, theme::FS_BODY, color)));
        }
    });
}

/// Tears the whole overlay down on leaving the TalentPicker state.
pub fn despawn_picker(mut commands: Commands, root: Query<Entity, With<TalentPickerRoot>>) {
    for entity in &root {
        commands.entity(entity).despawn();
    }
}
