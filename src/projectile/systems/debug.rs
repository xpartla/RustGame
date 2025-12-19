use bevy::color::palettes::css::RED;
use bevy::prelude::{Gizmos, Query, With};
use crate::core::components::WorldPosition;
use crate::projectile::components::{Hitbox, Projectile};

pub fn draw_projectile_gizmos(
    mut gizmos: Gizmos,
    projectiles: Query<(&WorldPosition, &Hitbox), With<Projectile>>,
) {
    for(pos, hitbox) in &projectiles {
        gizmos.circle_2d(
            pos.0,
            hitbox.radius,
            RED,
        );
    }
}