// GameState — the top-level application state machine (Phase 0).
//
// Added as the foundation for pausing, menus, and modal overlays. Gameplay-simulation systems
// are gated on `in_state(GameState::InRun)` so that entering any other state freezes the world
// without having to touch each system individually.
//
// For now the app boots straight into `InRun` (the default) so there is no visible change from
// the prototype — there is no menu yet.
//
// TODO(Phase 8): default to `Menu` once the main-menu / character-select flow exists, and move
//                entity spawning (player, map) from `Startup` to `OnEnter(GameState::InRun)`.
// TODO(Phase 2): push/pop `TalentPicker` from the level-up flow.
// TODO(Phase 9): push/pop `Merchant` from merchant nodes.

use bevy::prelude::States;

// Most variants are reserved for later phases (menus, pause, overlays) and are not yet
// constructed — the app only ever sits in `InRun` today. Allow until those flows land.
#[allow(dead_code)]
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    /// Main menu. Not yet implemented.
    Menu,
    /// Hero selection before a run starts. Not yet implemented.
    CharacterSelect,
    /// Active gameplay — the only state the current systems run in.
    #[default]
    InRun,
    /// Gameplay frozen, world preserved. Not yet implemented.
    Paused,
    /// Run ended (death or act-3 clear). Not yet implemented.
    GameOver,
    /// Level-up talent-choice overlay. Wired in Phase 2.
    TalentPicker,
    /// Merchant interaction overlay. Wired in Phase 9.
    Merchant,
}
