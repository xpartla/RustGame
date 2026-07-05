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

use bevy::prelude::*;

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
    /// Encounter-cleared branch picker (Phase 7). Freezes the InRun world like the TalentPicker
    /// while the player chooses the next act-graph node.
    MapSelect,
    /// Merchant interaction overlay. Wired in Phase 9.
    Merchant,
}

/// A snapshot of a finished run, captured the moment it ends (the player dies, or Act 3 is cleared)
/// — before the run's entities/resources are torn down, so the game-over screen can render it even
/// though the live run no longer exists. Written by `player_death` (defeat) and
/// `handle_encounter_complete` (Act-3 victory); read by `ui/screens/game_over.rs`. Cleared by the
/// run-reset primitive (Phase 7.5B) when a new run begins. `act`/`node_column` are `None` for a death
/// in a runless world (e.g. a headless arena scenario), which has no `RunState`.
#[derive(Resource, Debug, Clone)]
pub struct GameOverSummary {
    pub victory: bool,
    pub hero_id: String,
    pub level: u32,
    pub act: Option<u8>,
    pub node_column: Option<usize>,
}
