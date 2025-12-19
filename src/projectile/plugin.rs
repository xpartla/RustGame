use bevy::app::App;
use bevy::prelude::{IntoScheduleConfigs, Plugin, Update};
use crate::projectile::systems::collision_detection::projectile_hits_enemies;
use crate::projectile::systems::debug::draw_projectile_gizmos;
use crate::projectile::systems::projectile_lifetime::projectile_lifetime;

pub struct ProjectilePlugin;
impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                projectile_hits_enemies,
                projectile_lifetime.after(projectile_hits_enemies),
                draw_projectile_gizmos
                )
        );
    }
}