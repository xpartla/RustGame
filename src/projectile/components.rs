use bevy::math::Vec2;
use bevy::prelude::{Component, Entity};
use bevy::time::Timer;
use crate::ability::effects::ResolvedEffect;
use crate::core::components::Faction;

// This module now serves two entity kinds, both marked `Projectile` and despawned by
// `projectile_lifetime`:
//   1. Transient attack-VFX flashes (melee cone) — a shape + Lifetime, no motion. Damage is
//      resolved instantly by the ability system, not here.
//   2. Travelling projectiles (Fireblast, Frostbolt) — carry `ProjectileMotion` + a
//      `ProjectilePayload`; `move_projectiles` integrates them and `projectile_collision` applies
//      the payload's baked effects on impact. Only entities with `ProjectileMotion` are moved.

/// Marker for a projectile entity (transient VFX flash or a travelling projectile).
#[derive(Component)]
pub struct Projectile;

/// Travelling projectile motion + collision state.
#[derive(Component)]
pub struct ProjectileMotion {
    pub velocity: Vec2,
    /// Projectile collision radius; added to the target's radius at impact.
    pub radius: f32,
    /// Remaining enemies it can pass through after a hit (0 ⇒ despawn on the next hit).
    pub pierce_remaining: u32,
}

/// The baked effects a travelling projectile applies on impact, plus who fired it, which faction
/// it may hit, and which actors it already struck (so a piercing shot never double-hits one).
#[derive(Component)]
pub struct ProjectilePayload {
    pub source: Entity,
    /// The faction this projectile collides with — the opposite of the caster's (Phase 5). A
    /// player shot hits `Hostile`; an enemy shot hits `Friendly` (the player).
    pub target_faction: Faction,
    pub effects: Vec<ResolvedEffect>,
    pub already_hit: Vec<Entity>,
}

#[derive(Component)]
pub struct Lifetime {
    pub timer: Timer,
}

/// Circle hitbox shape (used both for the instant overlap test and for drawing the swing).
#[derive(Component)]
pub struct CircleHitbox {
    pub radius: f32,
}

/// Cone hitbox shape: everything within `radius` and `half_angle` of the forward direction.
#[derive(Component)]
pub struct ArcHitbox {
    pub radius: f32,
    pub half_angle: f32,
}
