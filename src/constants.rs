pub const PLAYER_SPEED: f32 = 35.0;
pub const FLOW_RADIUS: i32 = 50;
pub const TILE_SIZE: f32 = 32.0;
pub const PLAYER_HEALTH: f32 = 100.0;
/// Player body radius: logic hurtbox (core::components::Hurtbox) and visual circle share it.
pub const PLAYER_RADIUS: f32 = 25.0;

// Per-type enemy stats (health, speed, attack damage/range/cooldown, size, color) live in
// enemy/archetypes.rs. Only cross-type tuning stays here.
pub const ENEMY_ATTACK_FLASH_SECS: f32 = 0.15;

// Attack VFX tuning. Damage/range/cooldown now come from ability RON (assets/abilities/).
// This is only the lifetime of the transient hitbox-flash gizmo entity.
pub const ATTACK_LIFETIME: f32 = 0.1;

// XP / leveling. Per-enemy-type XP rewards live in enemy/archetypes.rs (`xp_value`); the level
// curve is global: XP to advance from `level` to the next = XP_FIRST_LEVEL + (level-1)*XP_LEVEL_STEP.
pub const XP_FIRST_LEVEL: u32 = 10;
pub const XP_LEVEL_STEP: u32 = 5;