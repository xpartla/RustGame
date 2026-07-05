// Merchant overlay (Phase 7.5E) — the shop shown in `GameState::Merchant`.
//
// Display only; input is logic-side (talent/systems/merchant.rs::handle_merchant_input). Lists the
// player's acquired talents with numbers matching the remove keys, and re-renders when the talent set
// changes (after a remove). Never runs headless.

use bevy::prelude::*;

use crate::player::components::Player;
use crate::talent::assets::{TalentDef, TalentLibrary};
use crate::talent::components::AcquiredTalents;
use crate::ui::theme::{self, text};

#[derive(Component)]
pub struct MerchantRoot;

/// The node whose children are the per-talent rows; rebuilt when AcquiredTalents changes.
#[derive(Component)]
pub struct MerchantList;

/// Marks a single talent row so a re-render can clear the previous rows.
#[derive(Component)]
pub struct MerchantRow;

pub fn spawn_merchant(mut commands: Commands) {
    commands
        .spawn((MerchantRoot, theme::overlay_root(), BackgroundColor(theme::OVERLAY_BG)))
        .with_children(|root| {
            root.spawn(text("MERCHANT", theme::FS_TITLE, theme::ACCENT));
            root.spawn(text("Your talents:", theme::FS_HEADING, theme::TITLE));
            root.spawn((
                MerchantList,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
            ));
            root.spawn(text(
                "digit — remove one     ·     T — trade 3 for 1 higher     ·     Esc — leave",
                theme::FS_HINT,
                theme::HINT,
            ));
        });
}

/// Rebuilds the talent list whenever the player's `AcquiredTalents` changes (e.g. after a remove).
pub fn render_merchant(
    mut commands: Commands,
    acquired: Query<&AcquiredTalents, (With<Player>, Changed<AcquiredTalents>)>,
    list: Query<Entity, With<MerchantList>>,
    rows: Query<Entity, With<MerchantRow>>,
    library: Res<TalentLibrary>,
    defs: Res<Assets<TalentDef>>,
) {
    let Ok(acquired) = acquired.single() else {
        return; // no change this frame
    };
    let Ok(list) = list.single() else {
        return; // overlay not spawned yet
    };
    for row in &rows {
        commands.entity(row).despawn();
    }
    commands.entity(list).with_children(|parent| {
        if acquired.entries.is_empty() {
            parent.spawn((MerchantRow, text("(no talents to trade)", theme::FS_BODY, theme::DIM)));
            return;
        }
        for (i, (id, count)) in acquired.entries.iter().enumerate() {
            let def = library.get(id).and_then(|h| defs.get(h));
            let name = def.map(|d| d.display_name.clone()).unwrap_or_else(|| id.clone());
            let color = def.map(|d| theme::rarity_color(&d.rarity)).unwrap_or(theme::TEXT);
            let label = if *count > 1 {
                format!("{}.   {}  ×{}", i + 1, name, count)
            } else {
                format!("{}.   {}", i + 1, name)
            };
            parent.spawn((MerchantRow, text(label, theme::FS_BODY, color)));
        }
    });
}

pub fn despawn_merchant(mut commands: Commands, root: Query<Entity, With<MerchantRoot>>) {
    for e in &root {
        commands.entity(e).despawn();
    }
}
