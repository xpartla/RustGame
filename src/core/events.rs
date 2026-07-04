use bevy::prelude::{Entity, Event};

/// Element tags on a `DamageEvent`. Used by the status effect system (Phase 3) to
/// trigger cross-element cancellations (Fire removes Frostbite, Frost removes Blaze).
/// Existing callers pass an empty `tags` slice — the field is purely additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageTag {
    Physical,
    Fire,
    Frost,
    Holy,
    Shadow,
    Arcane,
}

/// Request to deal `amount` damage to `target`. Any system (attacks, projectile
/// collisions, hazards, DoTs) emits this; `apply_damage` is the single place that
/// mutates `Health`. `source` records who caused it (for future attribution: reflect,
/// thorns, kill credit / XP).
///
/// `tags` — added in Phase 0. All existing callers pass an empty slice; the field is
/// read only by status/systems/cross_interact.rs (Phase 3).
#[derive(Event)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
    pub source: Entity,
    pub tags: Vec<DamageTag>,
}

/// Request to restore `amount` health to `target`. The healing counterpart to `DamageEvent`:
/// any system (pickups, regen, abilities) emits this; `apply_heal` is the single place that
/// adds to `Health`, clamping to `Health.max`.
#[derive(Event)]
pub struct HealEvent {
    pub target: Entity,
    pub amount: f32,
}

/// Request to award `amount` experience to `target`. Emitted on a kill (`enemy_death`, crediting
/// the killer via `LastHitBy`); `gain_experience` is the single place that mutates `Experience`.
/// `target`-based for future-proofing — only entities with an `Experience` component (the player)
/// actually gain XP; for anyone else it's a no-op.
#[derive(Event)]
pub struct GainXpEvent {
    pub target: Entity,
    pub amount: u32,
}

/// Fired by `gain_experience` each time the player crosses a level threshold. The hook for
/// level-up rewards (currently log-only; later the talent system).
#[derive(Event)]
pub struct LevelUpEvent {
    pub level: u32,
}
