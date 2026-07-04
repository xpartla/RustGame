use bevy::app::App;
use bevy::prelude::{IntoScheduleConfigs, Plugin, Update, in_state};
use crate::game::state::GameState;
use crate::projectile::systems::projectile_lifetime::projectile_lifetime;

// Presentation note: draw_circle_attack_gizmos / draw_arc_attack_gizmos are registered by
// game::presentation::PresentationPlugin with the same schedule + gating as before.

pub struct ProjectilePlugin;
impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            projectile_lifetime.run_if(in_state(GameState::InRun)),
        );
    }
}
