use bevy::prelude::{Component, Resource, Timer};

/// What a pickup does when the player collects it. Extensible: add `Xp(u32)`,
/// `Currency(u32)`, `Buff(...)` here and a matching arm in `collect_pickups`.
#[derive(Clone, Copy)]
pub enum PickUpKind {
    /// Restores this much `Health` (clamped to max via `HealEvent` → `apply_heal`).
    Heal(f32),
    /// Grants this many `hero::components::Charges` (Phase 9.4 — Druid's Bloom: "your next animal
    /// form attack is enhanced"). A no-op if the collector has no `Charges` component (any
    /// non-Charges hero — the flower just vanishes).
    Enhance(u32),
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
