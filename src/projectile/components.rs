use bevy::prelude::{Bundle, Component, Entity, Vec2};
use bevy::time::{Timer, TimerMode};
use crate::core::components::{Velocity, WorldPosition};

#[derive(Component)]
pub struct Projectile;

#[derive(Component)]
pub struct Damage(pub f32);

#[derive(Component)]
pub struct Lifetime{
    pub timer: Timer,
}

// #[derive(Component)]
// pub struct Hitbox {
//     pub radius: f32,
// }

#[derive(Component)]
pub struct Source {
    pub entity: Entity,
}

#[derive(Component)]
pub struct CircleHitbox {
    pub radius: f32,
}

#[derive(Component)]
pub struct ArcHitbox{
    pub radius: f32,
    pub half_angle: f32,
}

#[derive(Bundle)]
pub struct ProjectileBundle {
    pub projectile: Projectile,
    pub world: WorldPosition,
    pub vel: Velocity,
    pub damage: Damage,
    pub lifetime: Lifetime,
    pub source: Source,
}

impl ProjectileBundle {
    pub fn new(
        pos:Vec2,
        vel:Vec2,
        damage:f32,
        lifetime:f32,
        source:Entity,
    ) -> Self {
        Self {
            projectile: Projectile,
            world: WorldPosition(pos),
            vel: Velocity(vel),
            damage: Damage(damage),
            lifetime: Lifetime {
                timer: Timer::from_seconds(lifetime, TimerMode::Once),
            },
            source: Source { entity: source},
        }
    }
}