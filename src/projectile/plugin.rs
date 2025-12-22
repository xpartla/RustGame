use bevy::app::App;
use bevy::prelude::{IntoScheduleConfigs, Plugin, Update};
use crate::projectile::systems::collision_detection::{projectile_arc_hit_enemies, projectile_circle_hits_enemies};
use crate::projectile::systems::debug::{draw_arc_attack_gizmos, draw_circle_attack_gizmos};
use crate::projectile::systems::projectile_lifetime::projectile_lifetime;

pub struct ProjectilePlugin;
impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                projectile_circle_hits_enemies,
                projectile_arc_hit_enemies,
                projectile_lifetime.after(projectile_circle_hits_enemies).after(projectile_arc_hit_enemies),
                draw_circle_attack_gizmos,
                draw_arc_attack_gizmos,
                )
        );
    }
}