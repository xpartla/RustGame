use bevy::prelude::SystemSet;

/// Orders the per-frame combat chain so a hit fully resolves within a single frame:
/// emit `DamageEvent` → apply it to `Health` → status ticks → death.
/// Full order: CombatSet::Damage → CombatSet::Apply → StatusSet::Tick →
///             StatusSet::CrossInteract → CombatSet::Death.
/// Configured (chained) in `CorePlugin`; systems opt in with `.in_set(..)`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CombatSet {
    /// Systems that emit `DamageEvent` (player attacks, enemy contact, hazards, DoTs).
    Damage,
    /// `apply_damage` + `apply_heal` — single consumers that mutate `Health`.
    Apply,
    /// Death handling that reads post-damage `Health` (enemy/player death).
    Death,
}

/// Status effect processing, inserted between CombatSet::Apply and CombatSet::Death.
/// Added in Phase 3 (status module). Stub-declared here so the ordering is visible
/// from day one even before the systems exist.
///
/// TODO(Phase 3): configure in CorePlugin:
///   .configure_sets(Update,
///     (CombatSet::Damage, CombatSet::Apply,
///      StatusSet::Tick, StatusSet::CrossInteract,
///      CombatSet::Death).chain())
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum StatusSet {
    /// Advance DoT timers, emit DamageEvent for periodic effects (bleed, blaze ticks).
    Tick,
    /// Consume DamageEvent.tags to remove element-cancelled effects (fire removes frostbite).
    CrossInteract,
}
