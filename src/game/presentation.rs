// PresentationPlugin — every system that needs a window, GPU, or render assets.
//
// The gameplay simulation (GameLogicPlugin) spawns entities with logic components only;
// this plugin dresses them up. The split exists so the headless sim harness (src/sim/) can
// run the full game loop without a renderer (WSL has no GPU device). Registration here is a
// 1:1 move of what the domain plugins used to register — schedules, sets, and run
// conditions are preserved exactly.
//
// Contents:
//   - CameraPlugin, UiPlugin (whole plugins — camera follow, cursor gizmos, talent picker)
//   - render_map        (Startup, after generate_map — floor + obstacle tiles)
//   - attach_*_visuals  (Added<T> reactors that insert Transform + Mesh2d + material)
//   - sync_transform / apply_facing_rotation (logical position/facing → Transform)
//   - debug gizmos      (health bars, facing line, attack flashes, attack-shape outlines)

use bevy::prelude::*;
use crate::camera::CameraPlugin;
use crate::core::systems::debug::draw_health_bars;
use crate::core::systems::grid_sync::world_to_grid;
use crate::core::systems::render_sync::{apply_facing_rotation, sync_transform};
use crate::enemy::systems::debug::draw_enemy_attack_flash;
use crate::enemy::systems::visuals::attach_enemy_visuals;
use crate::game::state::GameState;
use crate::pickup::systems::visuals::attach_pickup_visuals;
use crate::player::systems::debug::draw_player_facing;
use crate::player::systems::visuals::attach_player_visuals;
use crate::projectile::systems::debug::{draw_arc_attack_gizmos, draw_circle_attack_gizmos};
use crate::projectile::systems::visuals::attach_projectile_visuals;
use crate::status::systems::visuals::tint_status_effects;
use crate::ui::UiPlugin;
use crate::world::systems::generate_map::generate_map;
use crate::world::systems::render_map::render_map;

pub struct PresentationPlugin;

impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((CameraPlugin, UiPlugin));

        app.add_systems(Startup, render_map.after(generate_map));

        // Visual dress-up for logic-spawned entities. Ungated so an entity spawned on the
        // last frame of a state is never missed (Added<T> is relative to the system's last run).
        app.add_systems(
            Update,
            (
                attach_player_visuals,
                attach_enemy_visuals,
                attach_pickup_visuals,
                attach_projectile_visuals,
            ),
        );

        // Status tinting: recolor enemies by their active status effect (Phase 4).
        app.add_systems(Update, tint_status_effects.run_if(in_state(GameState::InRun)));

        // Logical position/facing → Transform. Same ordering + gating they had in CorePlugin.
        app.add_systems(
            Update,
            (
                sync_transform.after(world_to_grid),
                apply_facing_rotation.after(world_to_grid),
            )
                .run_if(in_state(GameState::InRun)),
        );

        // Debug gizmos, registered exactly as their domain plugins used to.
        app.add_systems(PostUpdate, (draw_health_bars, draw_player_facing, draw_enemy_attack_flash));
        app.add_systems(
            Update,
            (draw_circle_attack_gizmos, draw_arc_attack_gizmos).run_if(in_state(GameState::InRun)),
        );
    }
}
