use bevy::math::Vec2;
use bevy::prelude::{Bundle, Commands, Entity, Res, ResMut};
use bevy::time::Time;
use rand::Rng;
use crate::constants::TILE_SIZE;
use crate::core::components::{Facing, GridPosition, Health, Hurtbox, LastHitBy, Velocity, WorldPosition};
use crate::enemy::archetypes::{pick, EnemyArchetype};
use crate::enemy::components::{
    AttackCooldown, AttackStats, Enemy, EnemyAppearance, EnemySpawner, MoveSpeed, XpReward,
};
use crate::world::components::TileMap;

/// The full logic component set for an enemy of the given archetype at a grid tile.
/// Shared by the timed spawner and the sim harness (sim/) so both spawn identical enemies.
/// Visuals (Transform/Mesh2d/material) are attached separately by the presentation layer,
/// keyed off the EnemyAppearance data included here.
pub fn enemy_bundle(archetype: &EnemyArchetype, grid: GridPosition) -> impl Bundle {
    let world = Vec2::new(grid.x as f32 * TILE_SIZE, grid.y as f32 * TILE_SIZE);
    (
        Enemy,
        Health::new(archetype.max_health),
        MoveSpeed(archetype.speed),
        AttackStats { damage: archetype.attack_damage, range: archetype.attack_range },
        AttackCooldown::new(archetype.attack_cooldown),
        XpReward(archetype.xp_value),
        // Tracks the killer for XP credit; no one has hit it yet.
        LastHitBy(Entity::PLACEHOLDER),
        grid,
        WorldPosition(world),
        Velocity::default(),
        Facing(Vec2::default()),
        // Same source value feeds the logic hurtbox and the visual size.
        Hurtbox { radius: archetype.radius },
        EnemyAppearance {
            shape: archetype.shape,
            radius: archetype.radius,
            color: archetype.color,
        },
    )
}

pub fn spawn_enemy_over_time(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<EnemySpawner>,
    map: Res<TileMap>,
){
    spawner.timer.tick(time.delta());
    if !spawner.timer.finished() {
        return;
    }

    let mut rng = rand::thread_rng();
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let dist = spawner.radius as f32;

    let x = (angle.cos() * dist) as i32;
    let y = (angle.sin() * dist) as i32;

    // Don't spawn inside a wall — the enemy would be stuck (no flow direction, blocked movement).
    // Skip this tick; the next one rolls a fresh angle.
    if map.is_blocked(GridPosition { x, y }) {
        return;
    }

    let archetype = pick(&mut rng);
    commands.spawn(enemy_bundle(&archetype, GridPosition { x, y }));
}
