pub const PLAYER_SPEED: f32 = 35.0;
pub const FLOW_RADIUS: i32 = 50;
pub const TILE_SIZE: f32 = 32.0;
pub const PLAYER_HEALTH: f32 = 100.0;
/// Player body radius: logic hurtbox (core::components::Hurtbox) and visual circle share it.
pub const PLAYER_RADIUS: f32 = 25.0;

// Per-type enemy stats (health, speed, size, color, scaling) live in the enemy RON files
// (assets/enemies/*.enemy.ron → EnemyDef); contact damage/range/cooldown live in the enemy
// contact abilities (assets/abilities/*_contact.ability.ron). Only cross-type tuning stays here.
pub const ENEMY_ATTACK_FLASH_SECS: f32 = 0.15;

// Attack VFX tuning. Damage/range/cooldown now come from ability RON (assets/abilities/).
// This is only the lifetime of the transient hitbox-flash gizmo entity.
pub const ATTACK_LIFETIME: f32 = 0.1;

// Persistent-zone occupant tick cadence (Phase 6). Zone DoT/regen land in discrete 1 Hz ticks
// (`damage_per_second` per tick), not per-frame — deterministic, no f32 drift. Per-zone radius,
// duration, dps, and regen come from the ability RON (assets/abilities/ → ZoneSpec + params).
pub const ZONE_TICK_INTERVAL: f32 = 1.0;
/// Consecrated Ground's `consecrated_ground_count_scaling_rare` talent (Phase 9.3): +15% damage
/// per additional enemy standing inside, per tick.
pub const CONSECRATED_COUNT_SCALING_FRACTION: f32 = 0.15;

// XP / leveling. Per-enemy-type XP rewards live in the enemy RON files (`xp_value`); the level
// curve is global: XP to advance from `level` to the next = XP_FIRST_LEVEL + (level-1)*XP_LEVEL_STEP.
pub const XP_FIRST_LEVEL: u32 = 10;
pub const XP_LEVEL_STEP: u32 = 5;

// Companion minion tuning (Phase 9.2, §8.1(3) `summon`). Attack damage/range/cooldown are its own
// AbilityDef (assets/abilities/companion_attack.ability.ron); these are the body itself — how
// tough/fast/big/far-seeking it is. Not exposed as ability params (no talent scales a minion's
// body directly today; its attack numbers are what talents would reasonably touch).
pub const MINION_HEALTH: f32 = 20.0;
pub const MINION_SPEED: f32 = 45.0;
pub const MINION_RADIUS: f32 = 10.0;
/// How far a minion will notice and chase a Hostile actor; beyond this it idles at the spot it
/// was summoned/left at.
pub const MINION_SEEK_RANGE: f32 = 300.0;
/// Once within this distance of its target, a minion holds position instead of continuing to
/// close — comfortably inside `companion_attack`'s 50-unit range, so it can actually swing instead
/// of oscillating past a (possibly stationary) target at melee range every frame.
pub const MINION_ENGAGE_RANGE: f32 = 35.0;