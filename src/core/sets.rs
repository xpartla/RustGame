use bevy::prelude::SystemSet;

/// Pins the movement pipeline to the front of the frame (Phase 3.1 hardening):
/// decide where everyone wants to go, then integrate positions — all before any combat
/// resolves. Without this pin, the loose movement systems were ordered by the scheduler's
/// tie-break, so merely *adding* a system in a later phase could reorder them and nudge
/// every position in the golden-master baseline (it happened twice within Phase 3).
/// Full frame order: MovementSet::Intent → MovementSet::Integrate → CombatSet::Damage → …
/// Configured (chained, before CombatSet::Damage) in `CorePlugin`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MovementSet {
    /// Velocity/facing setters: player input, flow-field rebuild + enemy steering.
    Intent,
    /// Position integration: `apply_velocity` → `world_to_grid`.
    Integrate,
}

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
/// Configured into the chain by CorePlugin since Phase 3; the systems live in
/// status/systems/ (see StatusPlugin for the per-set membership).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum StatusSet {
    /// Advance DoT timers, emit DamageEvent for periodic effects (bleed, blaze ticks).
    Tick,
    /// Consume DamageEvent.tags to remove element-cancelled effects (fire removes frostbite).
    CrossInteract,
}
