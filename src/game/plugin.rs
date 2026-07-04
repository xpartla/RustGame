use bevy::prelude::*;
use crate::ability::AbilityPlugin;
use crate::camera::CameraPlugin;
use crate::core::CorePlugin;
use crate::enemy::EnemyPlugin;
use crate::game::state::GameState;
use crate::pickup::PickUpPlugin;
use crate::player::PlayerPlugin;
use crate::projectile::ProjectilePlugin;
use crate::run::rng::RunRng;
use crate::world::WorldPlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            // Top-level state machine. Boots into GameState::InRun (its default) so the
            // prototype behaves exactly as before; gameplay systems are gated on this state.
            .init_state::<GameState>()
            // Seeded RNG for run-deterministic systems (currently only map generation).
            // Phase 0 seeds from OS entropy each launch to preserve the prototype's per-launch
            // map variation. TODO(Phase 7): seed from the run-start flow's RunState.seed so a
            // run is reproducible and resumable.
            .insert_resource(RunRng::from_seed(rand::random::<u64>()))
            .add_plugins((
                CorePlugin,
                WorldPlugin,
                PlayerPlugin,
                AbilityPlugin,
                EnemyPlugin,
                ProjectilePlugin,
                PickUpPlugin,
                CameraPlugin,
            ));
    }
}