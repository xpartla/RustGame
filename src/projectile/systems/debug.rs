use bevy::color::palettes::css::RED;
use bevy::prelude::{Gizmos, Isometry2d, Query, Rot2, With};
use crate::core::components::WorldPosition;
use crate::player::components::Facing;
use crate::projectile::components::{ArcHitbox, CircleHitbox, Projectile};

pub fn draw_circle_attack_gizmos(
    mut gizmos: Gizmos,
    projectiles: Query<(&WorldPosition, &CircleHitbox), With<Projectile>>,
) {
    for(pos, hitbox) in &projectiles {
        gizmos.circle_2d(
            pos.0,
            hitbox.radius,
            RED,
        );
    }
}


pub fn draw_arc_attack_gizmos(
    mut gizmos: Gizmos,
    projectiles: Query<(&WorldPosition, &ArcHitbox, &Facing), With<Projectile>>,
) {
    for (pos, arc, facing) in &projectiles {
        let forward_angle = facing.0.y.atan2(facing.0.x);
        let angle = forward_angle - std::f32::consts::FRAC_PI_4 * 3.0;
        let center = pos.0 + facing.0;

        let iso = Isometry2d::new(
            center,
            Rot2::radians(angle),
        );

        gizmos.arc_2d(
            iso,
            arc.half_angle * 2.0,
            arc.radius,
            RED,
        );
    }
}

