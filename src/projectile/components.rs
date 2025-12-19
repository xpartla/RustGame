use bevy::prelude::{Bundle, Component, Entity, Vec2};
use bevy::time::{Timer, TimerMode};
use crate::core::components::{Velocity, WorldPosition};

#[derive(Component)]
pub struct Projectile;

#[derive(Component)]
pub struct Damage(pub u32);

#[derive(Component)]
pub struct Lifetime{
    pub timer: Timer,
}

#[derive(Component)]
pub struct Hitbox {
    pub radius: f32,
}

#[derive(Component)]
pub struct Source {
    pub entity: Entity,
}

#[derive(Bundle)]
pub struct ProjectileBundle {
    pub projectile: Projectile,
    pub world: WorldPosition,
    pub vel: Velocity,
    pub damage: Damage,
    pub lifetime: Lifetime,
    pub hitbox: Hitbox,
    pub source: Source,
}

impl ProjectileBundle {
    pub fn new(
        pos:Vec2,
        vel:Vec2,
        damage:u32,
        radius:f32,
        lifetime:f32,
        source:Entity,
    ) -> Self {
        Self {
            projectile: Projectile,
            world: WorldPosition(pos),
            vel: Velocity(vel),
            damage: Damage(damage),
            hitbox: Hitbox {radius},
            lifetime: Lifetime {
                timer: Timer::from_seconds(lifetime, TimerMode::Once),
            },
            source: Source { entity: source},
        }
    }
}