// UI domain (introduced in Phase 2).
//
// Owns no gameplay data — every screen reads from other domains (progression, talent, run) and
// renders. Input for the talent picker lives in progression/systems/offer.rs, not here; the UI
// only displays LevelUpFlowState.pending_offer.
//
// Module map:
//   plugin.rs           — UiPlugin: registers per-screen spawn/render/despawn systems.
//   screens/
//     talent_picker.rs  — the level-up "choose 1 of 3" overlay (GameState::TalentPicker).

pub mod plugin;
pub mod screens;

pub use plugin::UiPlugin;
