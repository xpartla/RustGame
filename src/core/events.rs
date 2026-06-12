use bevy::prelude::{Entity, Event};

/// Request to deal `amount` damage to `target`. Any system (attacks, projectile
/// collisions, hazards, DoTs) emits this; `apply_damage` is the single place that
/// mutates `Health`. `source` records who caused it (for future attribution: reflect,
/// thorns, kill credit / XP).
#[derive(Event)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
    pub source: Entity,
}

/// Request to restore `amount` health to `target`. The healing counterpart to `DamageEvent`:
/// any system (pickups, regen, abilities) emits this; `apply_heal` is the single place that
/// adds to `Health`, clamping to `Health.max`.
#[derive(Event)]
pub struct HealEvent {
    pub target: Entity,
    pub amount: f32,
}
