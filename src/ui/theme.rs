// Shared UI theme — palette, font sizes, and small spawn helpers used by every screen (Phase 7.5).
//
// The `ui/` ground rule (architecture §2) holds here too: this module owns no gameplay state. It is
// pure presentation vocabulary so all overlays and the HUD read as one visual system (rarity colors,
// overlay/panel chrome, bar widgets). Screens still read their data from logic resources/components
// and render with these helpers; input stays logic-side.
//
// Never runs headless — like every `ui/` module it is registered only under PresentationPlugin, so
// the golden master never sees it.

use bevy::prelude::*;

use crate::talent::assets::TalentRarity;

// ── Palette ────────────────────────────────────────────────────────────────────────────────────
/// Dimmed full-screen backdrop behind a modal overlay (talent picker, pause, menu, …).
pub const OVERLAY_BG: Color = Color::srgba(0.0, 0.0, 0.05, 0.82);
/// A raised panel / card surface on top of the overlay backdrop.
pub const PANEL_BG: Color = Color::srgba(0.10, 0.11, 0.16, 0.95);
/// A subtle inset track behind a bar fill.
pub const TRACK_BG: Color = Color::srgb(0.14, 0.16, 0.22);

pub const TITLE: Color = Color::srgb(0.85, 0.90, 0.98);
pub const TEXT: Color = Color::srgb(0.90, 0.90, 0.95);
pub const DIM: Color = Color::srgb(0.55, 0.55, 0.62);
pub const HINT: Color = Color::srgb(0.60, 0.60, 0.70);
/// Warm gold used for reward / level-up chrome.
pub const ACCENT: Color = Color::srgb(0.92, 0.85, 0.55);
/// Red used for danger / death / curse chrome.
pub const DANGER: Color = Color::srgb(0.90, 0.32, 0.32);

// Bar fills.
pub const HEALTH_FILL: Color = Color::srgb(0.30, 0.78, 0.36);
pub const XP_FILL: Color = Color::srgb(0.36, 0.62, 0.96);
pub const RESOURCE_FILL: Color = Color::srgb(0.40, 0.70, 0.95);
pub const BOSS_FILL: Color = Color::srgb(0.86, 0.24, 0.28);
/// A ready ability slot's accent; a cooling-down slot dims toward `TRACK_BG`.
pub const SLOT_READY: Color = Color::srgb(0.32, 0.40, 0.52);
pub const SLOT_COOLING: Color = Color::srgb(0.18, 0.20, 0.26);

// ── Font sizes ───────────────────────────────────────────────────────────────────────────────
pub const FS_TITLE: f32 = 40.0;
pub const FS_HEADING: f32 = 30.0;
pub const FS_BODY: f32 = 26.0;
pub const FS_HINT: f32 = 20.0;
pub const FS_SMALL: f32 = 16.0;

/// The color a talent/reward rarity is drawn in (shared by the picker, merchant, and any list).
pub fn rarity_color(rarity: &TalentRarity) -> Color {
    match rarity {
        TalentRarity::Common => Color::srgb(0.78, 0.80, 0.84),
        TalentRarity::Rare => Color::srgb(0.36, 0.62, 0.96),
        TalentRarity::Epic => Color::srgb(0.74, 0.46, 0.96),
    }
}

// ── Spawn helpers ──────────────────────────────────────────────────────────────────────────────

/// A text bundle at a given size + color. The single text primitive every screen builds on.
pub fn text(s: impl Into<String>, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (Text::new(s.into()), TextFont { font_size: size, ..default() }, TextColor(color))
}

/// The shared full-screen modal overlay root: absolute, centered column with row gaps. Pair it with
/// `BackgroundColor(OVERLAY_BG)` and the screen's own root marker component when spawning.
pub fn overlay_root() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: Val::Px(18.0),
        ..default()
    }
}

/// A padded panel / card node (a column of rows). Pair with `BackgroundColor(PANEL_BG)`.
pub fn panel() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: Val::Px(10.0),
        padding: UiRect::all(Val::Px(22.0)),
        ..default()
    }
}
