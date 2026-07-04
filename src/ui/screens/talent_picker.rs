// Talent picker overlay — the level-up "choose 1 of 3" screen (GameState::TalentPicker).
//
// Display only. It reads LevelUpFlowState.pending_offer (populated by
// progression/systems/offer.rs::refill_offer) and renders one row per option, resolving each
// talent id to its display name + rarity via TalentLibrary. Player input (1/2/3, Esc) is handled
// by progression/systems/offer.rs::handle_talent_choice, which mutates the flow state; this
// screen re-renders whenever that state changes.
//
// Structure: a full-screen root holds a static title, an OptionsContainer (whose children are
// rebuilt on each offer change), and a static footer hint.

use bevy::prelude::*;
use crate::progression::state::LevelUpFlowState;
use crate::talent::assets::{TalentDef, TalentLibrary};

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
        .spawn((
            TalentPickerRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.78)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("LEVEL UP  —  Choose a Talent"),
                TextFont { font_size: 40.0, ..default() },
                TextColor(Color::srgb(0.92, 0.85, 0.55)),
            ));
            root.spawn((
                OptionsContainer,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
            ));
            root.spawn((
                Text::new("1 / 2 / 3 to choose      ·      Esc to skip"),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.65, 0.65, 0.7)),
            ));
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
                Text::new("(no eligible talents — press Esc to skip)"),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
            return;
        }
        for (i, id) in offer.options.iter().enumerate() {
            let (name, rarity) = library
                .get(id)
                .and_then(|h| defs.get(h))
                .map(|d| (d.display_name.clone(), format!("{:?}", d.rarity)))
                .unwrap_or_else(|| (id.clone(), "?".to_string()));
            parent.spawn((
                OptionRow,
                Text::new(format!("{}.   {}   [{}]", i + 1, name, rarity)),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.95)),
            ));
        }
    });
}

/// Tears the whole overlay down on leaving the TalentPicker state.
pub fn despawn_picker(mut commands: Commands, root: Query<Entity, With<TalentPickerRoot>>) {
    for entity in &root {
        commands.entity(entity).despawn();
    }
}
