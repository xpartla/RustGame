use std::collections::HashMap;
use bevy::math::Vec2;
use bevy::prelude::{Component, Entity, Resource};
use crate::constants::TILE_SIZE;

#[derive(Component, Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

impl GridPosition {
    /// The tile a continuous world position falls in (round-to-nearest, matching `world_to_grid`).
    pub fn from_world(world: Vec2) -> Self {
        Self {
            x: (world.x / TILE_SIZE).round() as i32,
            y: (world.y / TILE_SIZE).round() as i32,
        }
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct WorldPosition(pub Vec2);

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

/// Multiplies how far an entity is moved per frame (1.0 = normal). Generic actor stat, owned by
/// whoever writes it — currently `status::resolve_actor_status` folds in frostbite's 0.8 slow;
/// later buffs/haste use the same channel. `apply_velocity` scales the integration step by it, so
/// the underlying `Velocity` (and any AI lerp toward it) is left intact. Absent ⇒ 1.0.
#[derive(Component, Debug, Copy, Clone)]
pub struct MoveSpeedModifier(pub f32);

/// Multiplies incoming damage (1.0 = normal). `apply_damage` reads it; `resolve_actor_status`
/// folds in frostbite's 1.1 amplify. Absent ⇒ 1.0.
#[derive(Component, Debug, Copy, Clone)]
pub struct DamageTakenModifier(pub f32);

/// Marker: the entity's velocity is not integrated this frame (root, stun). Present ⇒ frozen.
/// `apply_velocity` skips integration; the AI still updates `Velocity`, so movement resumes
/// cleanly when the marker is removed.
#[derive(Component, Debug)]
pub struct Immobilized;

/// Direction an entity is oriented toward (unit vector). Source of truth for visual
/// rotation (`apply_facing_rotation`) and, for the player, attack aim. Shared because both
/// the player (mouse aim) and enemies (movement direction) carry it.
#[derive(Component, Debug, Copy, Clone)]
pub struct Facing(pub Vec2);

/// Records the entity that most recently dealt damage to this one (set by `apply_damage` from
/// `DamageEvent.source`). Read for kill-credit — e.g. `enemy_death` awards XP to the killer.
/// Initialized to `Entity::PLACEHOLDER` until the first hit lands.
#[derive(Component)]
pub struct LastHitBy(pub Entity);

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self {
            current: max,
            max,
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct FlowField {
    pub cost: HashMap<GridPosition, u32>,
    pub direction: HashMap<GridPosition, Vec2>,
}