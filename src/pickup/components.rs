use bevy::prelude::{Component, Resource, Timer};

/// What a pickup does when the player collects it. Extensible: add `Xp(u32)`,
/// `Currency(u32)`, `Buff(...)` here and a matching arm in `collect_pickups`.
#[derive(Clone, Copy)]
pub enum PickUpKind {
    /// Restores this much `Health` (clamped to max via `HealEvent` → `apply_heal`).
    Heal(f32),
}

/// Marker + payload for a collectible lying in the world.
#[derive(Component)]
pub struct PickUp {
    pub kind: PickUpKind,
}

/// Drives timed ambient spawning of pickups around the player (mirrors `EnemySpawner`).
#[derive(Resource)]
pub struct PickUpSpawner {
    pub timer: Timer,
}
