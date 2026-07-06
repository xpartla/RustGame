use bevy::prelude::{any_with_component, not, App, IntoScheduleConfigs, OnEnter, Plugin};
use crate::game::state::GameState;
use crate::player::components::Player;
use crate::world::components::TileMap;
use crate::world::systems::generate_map::generate_map;

// Presentation note: render_map (floor + obstacle meshes) is registered by
// game::presentation::PresentationPlugin, ordered after generate_map as before.

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // Phase 8, §5: moved from Startup to OnEnter(InRun), guarded like player/plugin.rs's
        // spawn_player (see its comment) — this placeholder map exists only so the windowed game
        // has *something* rendered before a real run starts; every real encounter regenerates the
        // room via load_encounter/generate_room, so this must never refire on an overlay round-trip
        // (which would silently reroll the layout and burn extra RunRng draws).
        app.init_resource::<TileMap>().add_systems(
            OnEnter(GameState::InRun),
            generate_map.run_if(not(any_with_component::<Player>)),
        );
    }
}
