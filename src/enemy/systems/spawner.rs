use bevy::asset::Assets;
use bevy::math::Vec2;
use bevy::prelude::{
    Circle, Commands, Entity, Mesh, Mesh2d, MeshMaterial2d, Rectangle, RegularPolygon, Res, ResMut,
    Transform,
};
use bevy::sprite::ColorMaterial;
use bevy::time::Time;
use rand::Rng;
use crate::constants::TILE_SIZE;
use crate::core::components::{Facing, GridPosition, Health, LastHitBy, Velocity, WorldPosition};
use crate::enemy::archetypes::{pick, EnemyShape};
use crate::enemy::components::{
    AttackCooldown, AttackStats, Enemy, EnemySpawner, MoveSpeed, XpReward,
};
use crate::world::components::TileMap;

pub fn spawn_enemy_over_time(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<EnemySpawner>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
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

    let world = Vec2::new(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE);

    let archetype = pick(&mut rng);

    let mesh = match archetype.shape {
        EnemyShape::Circle => meshes.add(Circle::new(archetype.radius)),
        EnemyShape::Square => {
            meshes.add(Rectangle::new(archetype.radius * 2.0, archetype.radius * 2.0))
        }
        EnemyShape::Triangle => meshes.add(RegularPolygon::new(archetype.radius, 3)),
    };
    let material = materials.add(archetype.color);

    commands.spawn((
        Enemy,
        Health::new(archetype.max_health),
        MoveSpeed(archetype.speed),
        AttackStats { damage: archetype.attack_damage, range: archetype.attack_range },
        AttackCooldown::new(archetype.attack_cooldown),
        XpReward(archetype.xp_value),
        // Tracks the killer for XP credit; no one has hit it yet.
        LastHitBy(Entity::PLACEHOLDER),
        GridPosition { x, y },
        WorldPosition(world),
        Velocity::default(),
        Facing(Vec2::default()),
        // z=1: above the background (z=0), below the player (z=2). sync_transform keeps x/y.
        Transform::from_xyz(world.x, world.y, 1.0),
        Mesh2d(mesh),
        MeshMaterial2d(material),
    ));
}
