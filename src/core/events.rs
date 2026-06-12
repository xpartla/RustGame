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
