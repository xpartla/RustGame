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
    /// Frostbolt's own innate identity (Phase 9.5, base kit — not a talent): "if the target is
    /// already affected by frostbite, generate a frost charge." Baked at cast time from a resolved
    /// param flag (mirrors the `follow_caster`/`slow_active` escape-hatch pattern), so it costs
    /// nothing for every other projectile. Read in `projectile_collision` BEFORE this hit's own
    /// `ApplyStatus(frostbite)` effect lands, so it only fires for a target frostbitten by a PRIOR
    /// cast, never this one. This is the projectile-impact talent/innate-effect gap the Phase 9.4
    /// as-built notes flagged for Mage completion.
    pub grants_frost_charge_on_frostbitten: bool,
    /// Fireblast's "explodes on impact" unique talent (Phase 9.5): `(damage, radius)` baked from
    /// `ActiveHooks` + resolved params at cast time (a talent picked up mid-flight doesn't
    /// retroactively alter an in-flight shot, the same rule every baked-at-cast-time field in this
    /// codebase follows). `None` for every other projectile / when the talent isn't acquired.
    pub explode_on_impact: Option<(f32, f32)>,
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
