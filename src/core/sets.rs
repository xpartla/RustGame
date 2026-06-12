use bevy::prelude::SystemSet;

/// Orders the per-frame combat chain so a hit fully resolves within a single frame:
/// emit `DamageEvent` → apply it to `Health` → handle death. Configured (chained) in
/// `CorePlugin`; systems opt in with `.in_set(..)`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CombatSet {
    /// Systems that emit `DamageEvent` (player attacks, later enemy contact, hazards).
    Damage,
    /// `apply_damage` — the single consumer that mutates `Health`.
    Apply,
    /// Death handling that reads post-damage `Health` (enemy/player death).
    Death,
}
