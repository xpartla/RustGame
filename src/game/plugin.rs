// GamePlugin — composition root.
//
// Split (testing-infrastructure stage 0) into two layers:
//   GameLogicPlugin    — the complete gameplay simulation. No render/window/GPU dependency;
//                        this is what the headless sim harness (src/sim/) runs under
//                        MinimalPlugins for integration tests and (later) balance sweeps.
//   PresentationPlugin — everything visual: camera, UI, meshes/materials attached to logic
//                        entities, map rendering, debug gizmos. Requires DefaultPlugins.
//
// The windowed game (src/main.rs) adds GamePlugin = logic + presentation. Behavior is
// unchanged from before the split.

use bevy::prelude::*;
use crate::ability::AbilityPlugin;
use crate::core::CorePlugin;
use crate::enemy::EnemyPlugin;
use crate::game::presentation::PresentationPlugin;
use crate::game::state::GameState;
use crate::pickup::PickUpPlugin;
use crate::player::PlayerPlugin;
use crate::progression::plugin::ProgressionPlugin;
use crate::projectile::ProjectilePlugin;
use crate::run::rng::RunRng;
use crate::status::plugin::StatusPlugin;
use crate::talent::plugin::TalentPlugin;
use crate::world::WorldPlugin;

/// The full gameplay simulation, free of render/window dependencies.
pub struct GameLogicPlugin;

impl Plugin for GameLogicPlugin {
    fn build(&self, app: &mut App) {
        app
            // Top-level state machine. Boots into GameState::InRun (its default) so the
            // prototype behaves exactly as before; gameplay systems are gated on this state.
            .init_state::<GameState>();
        // Seeded RNG for run-deterministic systems (map generation, band shuffles, offers,
        // death drops). Seeded from OS entropy each launch to preserve the prototype's
        // per-launch variation — unless a caller (the sim harness) already inserted a
        // fixed-seed RunRng before adding this plugin. TODO(Phase 7): seed from the
        // run-start flow's RunState.seed so a run is reproducible and resumable.
        if app.world().get_resource::<RunRng>().is_none() {
            app.insert_resource(RunRng::from_seed(rand::random::<u64>()));
        }
        app.add_plugins((
            CorePlugin,
            WorldPlugin,
            PlayerPlugin,
            AbilityPlugin,
            StatusPlugin,
            TalentPlugin,
            ProgressionPlugin,
            EnemyPlugin,
            ProjectilePlugin,
            PickUpPlugin,
        ));
    }
}

/// Logic + presentation: the plugin the windowed binary runs.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameLogicPlugin);
        app.add_plugins(PresentationPlugin);
    }
}
