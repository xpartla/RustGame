use std::collections::HashMap;
use bevy::math::Vec2;
use bevy::prelude::{Component, Entity, Resource, Timer, TimerMode};
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

/// Per-entity movement speed (world units/sec) — the raw dial WASD/AI multiplies a direction by.
/// Shared between the player (Phase 9.2 base_stats — `HeroDef.base_stats.move_speed`) and enemies
/// (set from `EnemyDef.base_stats.move_speed` at spawn). Was enemy-only before Phase 9.2.
#[derive(Component)]
pub struct MoveSpeed(pub f32);

/// Multiplies how far an entity is moved per frame (1.0 = normal). Generic actor stat, owned by
/// whoever writes it — currently `status::resolve_actor_status` folds in frostbite's 0.8 slow;
/// later buffs/haste use the same channel. `apply_velocity` scales the integration step by it, so
/// the underlying `Velocity` (and any AI lerp toward it) is left intact. Absent ⇒ 1.0.
#[derive(Component, Debug, Copy, Clone)]
pub struct MoveSpeedModifier(pub f32);

/// A second, independent speed multiplier `apply_velocity` folds in alongside
/// `MoveSpeedModifier` (1.0 = neutral). Kept separate rather than sharing that channel because
/// `MoveSpeedModifier` is entirely OWNED/overwritten each frame by `status::resolve_actor_status`
/// from the entity's OWN active statuses — a second, independently-driven source (Phase 9.2's
/// AMZ's "move faster while standing in your own zone" talent, driven by zone presence, not
/// status effects) would race that ownership if it wrote the same component. Managed by
/// `zone::systems::speed_bonus::resolve_zone_speed_bonus`. Absent ⇒ 1.0.
#[derive(Component, Debug, Copy, Clone)]
pub struct ZoneSpeedModifier(pub f32);

/// Multiplies incoming damage (1.0 = normal). `apply_damage` reads it; `resolve_actor_status`
/// folds in frostbite's 1.1 amplify. Absent ⇒ 1.0.
#[derive(Component, Debug, Copy, Clone)]
pub struct DamageTakenModifier(pub f32);

/// Multiplies incoming healing (1.0 = normal). `apply_heal` reads it (Phase 9.2 —
/// `bdk_passive_health_and_healing`'s "+healing taken%" half, folded by
/// `talent::systems::passives::resolve_health_and_healing`). Absent ⇒ 1.0.
#[derive(Component, Debug, Copy, Clone)]
pub struct HealingTakenModifier(pub f32);

/// The hero's pristine, un-talent-boosted max health (Phase 9.2), set once at
/// `player::systems::base_stats::apply_base_stats` alongside the real `Health.max`. The reference
/// point `talent::systems::passives::resolve_health_and_healing` recomputes
/// `bdk_passive_health_and_healing`'s max-health bonus FROM on every stack change, so re-acquiring
/// (or a future respec) never compounds against an already-boosted value.
#[derive(Component, Debug, Copy, Clone)]
pub struct BaseHealth(pub f32);

/// Multiplies damage an actor *deals* (1.0 = normal). The mirror of `DamageTakenModifier`, read on
/// the `DamageEvent.source` by `apply_damage`. Enemy scaling inserts it at spawn (depth > 0) so a
/// deeper enemy hits harder without per-depth damage numbers; absent ⇒ 1.0, so it is neutral at
/// depth 0 (Phase 5). Absent ⇒ 1.0.
#[derive(Component, Debug, Copy, Clone)]
pub struct DamageDealtModifier(pub f32);

/// Marker: the entity's velocity is not integrated this frame (root, stun). Present ⇒ frozen.
/// `apply_velocity` skips integration; the AI still updates `Velocity`, so movement resumes
/// cleanly when the marker is removed.
#[derive(Component, Debug)]
pub struct Immobilized;

/// Marker: the entity cannot cast, auto-cast, or stance-swap while present (stun's
/// `suppress_abilities`). Reconciled by `resolve_actor_status` like `Immobilized`, and consumed by
/// the ability execute/auto-cast systems and the hero input/stance systems (Phase 5). Applies to
/// player and enemy casters alike. Absent ⇒ may cast.
#[derive(Component, Debug)]
pub struct AbilitiesSuppressed;

/// A damage-absorbing pool (Phase 9.1, §8.1(5)). Consumed by `apply_damage` BEFORE `Health`,
/// between the `DamageTakenModifier` scaling and the health write — a hit larger than the pool
/// spills the remainder to `Health`. Multiple grants (`GainShieldEvent`, core/events.rs) stack
/// additively into this one component; it is removed once fully drained. No shipped content grants
/// one yet — bone shield / Ice Barrier / Purgatory are the first consumers (Phase 9.2/9.5).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Absorb {
    pub amount: f32,
}

/// Marker: this entity takes zero damage while present (`apply_damage` skips it entirely — the
/// incoming hit is discarded, not merely absorbed). Phase 9.2's Purgatory (BDK cheat-death) is the
/// first grantor; a generic primitive any future timed-immunity mechanic can reuse. Ticked down and
/// removed by `core::systems::invulnerability::tick_invulnerability`.
#[derive(Component, Debug)]
pub struct Invulnerable(pub Timer);

/// A one-shot positional impulse (Phase 9.1, §8.1(6)) that overrides an entity's `Velocity` for its
/// duration, then removes itself. Grip (pull toward a point — Abomination Limb) and knockback
/// (push away — a shockwave talent) are the same shape, differing only in the direction baked in at
/// construction. Resolved by `resolve_forced_movement`, which runs ahead of `apply_velocity` in
/// `MovementSet::Integrate` so the impulse still respects `TileMap` wall-sliding and overrides
/// whatever `MovementSet::Intent` (flow-field AI, WASD input) set that frame. No shipped ability
/// grants one yet — Abomination Limb's grip is the first consumer (Phase 9.2).
#[derive(Component, Debug, Clone)]
pub struct ForcedImpulse {
    pub velocity: Vec2,
    pub timer: Timer,
}

impl ForcedImpulse {
    /// A pull toward `to_point` from `from`, at `speed` world-units/sec, for `duration` seconds.
    /// The direction is resolved once (a one-shot impulse, not a continuous re-target).
    pub fn toward_point(from: Vec2, to_point: Vec2, speed: f32, duration: f32) -> Self {
        Self {
            velocity: (to_point - from).normalize_or_zero() * speed,
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }

    /// A push along `direction`, at `speed` world-units/sec, for `duration` seconds.
    pub fn knockback(direction: Vec2, speed: f32, duration: f32) -> Self {
        Self {
            velocity: direction.normalize_or_zero() * speed,
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

/// Logic collision radius for incoming hits (Phase 3.1). Read by projectile collision now;
/// enemy shots hitting the *player* (Phase 5) read the same component. Visual size lives in
/// presentation data (`EnemyAppearance`, the player mesh) — both are set from the same source
/// value, but gameplay must never read a presentation component.
#[derive(Component, Debug, Copy, Clone)]
pub struct Hurtbox {
    pub radius: f32,
}

/// Direction an entity is oriented toward (unit vector). Source of truth for visual
/// rotation (`apply_facing_rotation`) and, for the player, attack aim. Shared because both
/// the player (mouse aim) and enemies (movement direction) carry it.
#[derive(Component, Debug, Copy, Clone)]
pub struct Facing(pub Vec2);

/// Which side an actor fights for (Phase 5). An ability targets the faction *opposing* its
/// caster: the player (and player-side summons) are `Friendly`; enemies (and enemy summons) are
/// `Hostile`. The ability engine gathers candidates by faction instead of the old hardcoded
/// `With<Enemy>`, so enemy casts hit the player and player casts hit enemies through one path.
#[derive(Component, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Faction {
    Friendly,
    Hostile,
}

impl Faction {
    /// The faction an actor of this faction attacks.
    pub fn opposing(self) -> Faction {
        match self {
            Faction::Friendly => Faction::Hostile,
            Faction::Hostile => Faction::Friendly,
        }
    }
}

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