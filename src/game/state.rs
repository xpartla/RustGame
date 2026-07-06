// GameState — the top-level application state machine (Phase 0).
//
// Added as the foundation for pausing, menus, and modal overlays. Gameplay-simulation systems
// are gated on `in_state(GameState::InRun)` so that entering any other state freezes the world
// without having to touch each system individually.
//
// `InRun` stays the `#[default]` variant deliberately (Phase 8, D1/D4): the headless sim boots
// straight into it (no Login/Menu/CharacterSelect detour needed for the ~150 sim-driven tests),
// while the windowed boot (`GamePlugin`) immediately drives Login → Menu → CharacterSelect → run
// via a `Startup` system (`run/systems/menu.rs::enter_login`) — mirroring how `enter_main_menu`
// worked pre-Login. Player/map/level-flow spawn moved from `Startup` to `OnEnter(InRun)` in Phase
// 8 (§5 of docs/phase8-plan.md), guarded so the one-time boot seed doesn't refire on every
// overlay round-trip back into InRun.

use bevy::prelude::*;

// Most variants are reserved for later phases (menus, pause, overlays) and are not yet
// constructed — the app only ever sits in `InRun` today. Allow until those flows land.
#[allow(dead_code)]
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    /// Local-profile splash before the main menu (Phase 8, D4). Windowed boot only.
    Login,
    /// Main menu.
    Menu,
    /// Hero selection before a run starts.
    CharacterSelect,
    /// Active gameplay — the only state the current systems run in.
    #[default]
    InRun,
    /// Gameplay frozen, world preserved.
    Paused,
    /// Run ended (death or act-3 clear).
    GameOver,
    /// Level-up talent-choice overlay. Wired in Phase 2.
    TalentPicker,
    /// Encounter-cleared branch picker (Phase 7). Freezes the InRun world like the TalentPicker
    /// while the player chooses the next act-graph node.
    MapSelect,
    /// Merchant interaction overlay. Wired in Phase 7.5E.
    Merchant,
    /// Read-only run-history list, sorted by score (Phase 8).
    Scoreboard,
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
