use bevy::color::Color;
use rand::Rng;

/// Visual shape for an enemy type (built into a `Mesh2d` at spawn).
#[derive(Clone, Copy)]
pub enum EnemyShape {
    Circle,
    Triangle,
    Square,
}

/// Data-driven definition of an enemy type: stats + visuals + spawn weight. This is the single
/// source of truth for per-type values — they are copied onto per-entity components at spawn,
/// so systems read the entity's components, never these globals.
#[derive(Clone, Copy)]
pub struct EnemyArchetype {
    pub max_health: f32,
    pub speed: f32,
    pub attack_damage: f32,
    pub attack_range: f32,
    pub attack_cooldown: f32,
    pub radius: f32,
    pub color: Color,
    pub shape: EnemyShape,
    /// Relative spawn frequency (weighted random; not a probability).
    pub weight: u32,
}

/// The currently spawnable enemy types.
pub fn archetypes() -> [EnemyArchetype; 3] {
    [
        // Grunt — balanced baseline. Common.
        EnemyArchetype {
            max_health: 10.0,
            speed: 15.0,
            attack_damage: 5.0,
            attack_range: 28.0,
            attack_cooldown: 1.0,
            radius: 12.0,
            color: Color::srgb(0.85, 0.45, 0.10),
            shape: EnemyShape::Circle,
            weight: 6,
        },
        // Runner — fast and fragile, hits often but light. Medium.
        EnemyArchetype {
            max_health: 5.0,
            speed: 28.0,
            attack_damage: 3.0,
            attack_range: 24.0,
            attack_cooldown: 0.7,
            radius: 9.0,
            color: Color::srgb(0.90, 0.85, 0.20),
            shape: EnemyShape::Triangle,
            weight: 3,
        },
        // Brute — slow, tanky, hits hard. Rare.
        EnemyArchetype {
            max_health: 30.0,
            speed: 8.0,
            attack_damage: 12.0,
            attack_range: 32.0,
            attack_cooldown: 1.6,
            radius: 18.0,
            color: Color::srgb(0.80, 0.15, 0.15),
            shape: EnemyShape::Square,
            weight: 1,
        },
    ]
}

/// Weighted-random pick of an archetype.
pub fn pick(rng: &mut impl Rng) -> EnemyArchetype {
    let arches = archetypes();
    let total: u32 = arches.iter().map(|a| a.weight).sum();
    let mut roll = rng.gen_range(0..total);
    for archetype in &arches {
        if roll < archetype.weight {
            return *archetype;
        }
        roll -= archetype.weight;
    }
    arches[0]
}
