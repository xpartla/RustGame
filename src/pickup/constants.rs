// Pickup tuning. See `pickup/` for the feature; these are the only knobs.

/// Player within this distance (world units) of a pickup collects it.
pub const PICKUP_RADIUS: f32 = 24.0;

/// Health restored by a healing pack.
pub const HEAL_PACK_AMOUNT: f32 = 25.0;

/// Visual radius of the healing-pack mesh (a small green circle).
pub const HEAL_PACK_VISUAL_RADIUS: f32 = 8.0;

/// Seconds between ambient (timed) pickup spawns.
pub const PICKUP_SPAWN_SECS: f32 = 8.0;

/// Ambient pickups spawn on a ring this far (world units) from the player — far enough not to
/// be auto-collected, near enough to be reachable.
pub const PICKUP_SPAWN_MIN_DIST: f32 = 96.0;
pub const PICKUP_SPAWN_MAX_DIST: f32 = 256.0;

/// Chance (0.0..1.0) that an enemy drops a healing pack where it dies.
pub const ENEMY_DROP_CHANCE: f32 = 0.15;
