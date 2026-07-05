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

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use crate::ability::AbilityPlugin;
use crate::core::CorePlugin;
use crate::game::pause::toggle_pause;
use crate::enemy::EnemyPlugin;
use crate::game::presentation::PresentationPlugin;
use crate::game::state::GameState;
use crate::hero::HeroPlugin;
use crate::pickup::PickUpPlugin;
use crate::player::PlayerPlugin;
use crate::progression::plugin::ProgressionPlugin;
use crate::projectile::ProjectilePlugin;
use crate::run::rng::RunRng;
use crate::run::RunPlugin;
use crate::run::systems::menu::enter_main_menu;
use crate::status::plugin::StatusPlugin;
use crate::talent::plugin::TalentPlugin;
use crate::world::WorldPlugin;
use crate::zone::plugin::ZonePlugin;

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
            HeroPlugin,
            AbilityPlugin,
            StatusPlugin,
            TalentPlugin,
            ProgressionPlugin,
            EnemyPlugin,
            ProjectilePlugin,
            PickUpPlugin,
            ZonePlugin,
            // Run lifecycle (Phase 7). All its systems gate on a live run (CurrentEncounter/RunState);
            // with no run active — the headless sim's default, and the golden campaign — they are inert.
            RunPlugin,
        ));

        // Pause toggle (Phase 7.5B). Only runs on a frame where Esc is pressed; the golden campaign
        // never presses Esc, so it is byte-identical there.
        app.add_systems(Update, toggle_pause.run_if(input_just_pressed(KeyCode::Escape)));
    }
}

/// Logic + presentation: the plugin the windowed binary runs.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameLogicPlugin);
        app.add_plugins(PresentationPlugin);
        // Windowed-only boot to the main menu (Phase 7.5C, D1): replaces Phase 7's auto-start-run.
        // The game now boots Menu → CharacterSelect → run. Added by GamePlugin (windowed), NOT
        // GameLogicPlugin — the headless sim never runs it (Sim stays InRun), so the golden campaign
        // is byte-identical. Startup so the Menu transition applies before the first gated Update.
        app.add_systems(Startup, enter_main_menu);
    }
}
