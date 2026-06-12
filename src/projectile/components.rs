use bevy::prelude::Component;
use bevy::time::Timer;

// NOTE: provisional module. These are currently transient *attack VFX* entities — a melee
// swing spawns one purely so the gizmos can draw the hitbox shape for `Lifetime`. The actual
// damage is resolved instantly in the attack systems (see player/systems/attack.rs), not here.
// When real travelling projectiles (ranged attacks) are added, this module should grow a
// proper movement + collision system again (and likely be renamed/split).

/// Marker for a transient attack-VFX entity.
#[derive(Component)]
pub struct Projectile;

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
