pub const PLAYER_SPEED: f32 = 35.0;
pub const FLOW_RADIUS: i32 = 50;
pub const TILE_SIZE: f32 = 32.0;
pub const PLAYER_HEALTH: f32 = 100.0;

// Per-type enemy stats (health, speed, attack damage/range/cooldown, size, color) live in
// enemy/archetypes.rs. Only cross-type tuning stays here.
pub const ENEMY_ATTACK_FLASH_SECS: f32 = 0.15;
pub const ARC_BASE_DMG: f32 = 5.0;
pub const CIRCLE_BASE_DMG: f32 = 2.0;

// Attack tuning
pub const ATTACK_SPAWN_DISTANCE: f32 = 16.0;
pub const ATTACK_HITBOX_RADIUS: f32 = 20.0;
pub const ATTACK_LIFETIME: f32 = 0.1;