// UI domain (introduced in Phase 2).
//
// Owns no gameplay data — every screen reads from other domains (progression, talent, run) and
// renders. Input for the talent picker lives in progression/systems/offer.rs, not here; the UI
// only displays LevelUpFlowState.pending_offer.
//
// Module map:
//   plugin.rs           — UiPlugin: registers per-screen spawn/render/despawn systems.
//   theme.rs            — shared palette + spawn helpers (Phase 7.5A); every screen builds on it.
//   screens/
//     talent_picker.rs  — the level-up "choose 1 of 3" overlay (GameState::TalentPicker).
//     map_select.rs     — the act-graph branch picker (GameState::MapSelect).
//     hud.rs            — the persistent in-run HUD (GameState::InRun).

pub mod plugin;
pub mod screens;
pub mod theme;

pub use plugin::UiPlugin;
