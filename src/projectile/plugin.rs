use bevy::app::App;
use bevy::prelude::{IntoScheduleConfigs, Plugin, Update, in_state};
use crate::game::state::GameState;
use crate::projectile::systems::debug::{draw_arc_attack_gizmos, draw_circle_attack_gizmos};
use crate::projectile::systems::projectile_lifetime::projectile_lifetime;

pub struct ProjectilePlugin;
impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                projectile_lifetime,
                draw_circle_attack_gizmos,
                draw_arc_attack_gizmos,
                ).run_if(in_state(GameState::InRun))
        );
    }
}