use bevy::math::Vec2;
use bevy::prelude::{Bundle, Commands, Entity, Res, ResMut};
use bevy::time::Time;
use rand::Rng;
use crate::ability::components::{AbilityCooldown, AbilityInstance};
use crate::constants::TILE_SIZE;
use crate::core::components::{
    DamageDealtModifier, Facing, Faction, GridPosition, Health, Hurtbox, LastHitBy, MoveSpeed, Velocity, WorldPosition,
};
use crate::enemy::assets::{resolve_enemy_stats, EnemyDef, EnemyLibrary};
use crate::enemy::components::{AiBehavior, Enemy, EnemyAppearance, EnemySpawner, XpReward};
use crate::world::components::TileMap;
use bevy::asset::Assets;

/// The full logic component set for an enemy built from its `EnemyDef` at a grid tile and spawn
/// `depth`. Shared by the timed spawner and the sim harness so both spawn identical enemies.
/// Ability instances (contact melee, etc.) and the depth-scaling `DamageDealtModifier` are added
/// alongside by `spawn_enemy_from_def` — a bundle cannot carry child entities or a conditional
/// component. Visuals (Transform/Mesh2d/material) are attached by the presentation layer, keyed off
/// the `EnemyAppearance` data included here.
pub fn enemy_bundle(def: &EnemyDef, grid: GridPosition, depth: u32) -> impl Bundle {
    let stats = resolve_enemy_stats(def, depth);
    let world = Vec2::new(grid.x as f32 * TILE_SIZE, grid.y as f32 * TILE_SIZE);
    (
        Enemy,
        Health::new(stats.max_health),
        MoveSpeed(def.base_stats.move_speed),
        XpReward(stats.xp_value),
        // Tracks the killer for XP credit; no one has hit it yet.
        LastHitBy(Entity::PLACEHOLDER),
        grid,
        WorldPosition(world),
        Velocity::default(),
        Facing(Vec2::default()),
        // Same source value feeds the logic hurtbox and the visual size.
        Hurtbox { radius: def.base_stats.size_radius },
        EnemyAppearance {
            shape: def.appearance.shape,
            radius: def.base_stats.size_radius,
            color: def.appearance.color(),
        },
        // Enemies fight for the Hostile faction; player abilities target it (Phase 5).
        Faction::Hostile,
        AiBehavior::from_id(&def.ai_behavior, def.preferred_range),
    )
}

/// Spawns an enemy from its `EnemyDef`: the component bundle, one `AbilityInstance` per declared
/// ability (contact melee, ranged bolt, …), and — only at `depth > 0` — a `DamageDealtModifier`
/// for scaling. The ability instances are spawned *with* the enemy (not on `Added<Enemy>`) so the
/// contact ability is ready the very next frame, preserving the prototype's first-hit-on-contact.
pub fn spawn_enemy_from_def(
    commands: &mut Commands,
    def: &EnemyDef,
    grid: GridPosition,
    depth: u32,
) -> Entity {
    let enemy = commands.spawn(enemy_bundle(def, grid, depth)).id();
    let stats = resolve_enemy_stats(def, depth);
    if (stats.damage_mult - 1.0).abs() > 1e-6 {
        commands.entity(enemy).insert(DamageDealtModifier(stats.damage_mult));
    }
    for ability_id in &def.abilities {
        commands.spawn((
            AbilityInstance { def_id: ability_id.clone(), owner: enemy },
            // Start ready; execute re-reads the resolved "cooldown" param on the first cast.
            AbilityCooldown::new(0.0),
        ));
    }
    enemy
}

/// Ambient spawner: on each timer tick, weighted-picks a loaded `EnemyDef` and spawns it on a ring
/// around the origin. Rolls `thread_rng` (spawn variation is not seed-deterministic — scenarios
/// pause this timer; see docs/testing.md). Skips a tick if the chosen tile is blocked or no enemy
/// def has loaded yet.
pub fn spawn_enemy_over_time(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<EnemySpawner>,
    map: Res<TileMap>,
    library: Res<EnemyLibrary>,
    defs: Res<Assets<EnemyDef>>,
) {
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

    // Weighted pick over the loaded enemy defs.
    let loaded: Vec<&EnemyDef> = library.defs.values().filter_map(|h| defs.get(h)).collect();
    let total: u32 = loaded.iter().map(|d| d.spawn_weight).sum();
    if total == 0 {
        return; // no defs loaded yet
    }
    let mut roll = rng.gen_range(0..total);
    let mut chosen = loaded[0];
    for d in &loaded {
        if roll < d.spawn_weight {
            chosen = d;
            break;
        }
        roll -= d.spawn_weight;
    }

    spawn_enemy_from_def(&mut commands, chosen, GridPosition { x, y }, 0);
}
