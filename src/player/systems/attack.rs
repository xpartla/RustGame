use bevy::input::ButtonInput;
use bevy::prelude::{Commands, Entity, EventWriter, KeyCode, Query, Res, With};
use bevy::time::{Timer, TimerMode};
use crate::constants::{
    ARC_BASE_DMG, CIRCLE_BASE_DMG, ATTACK_SPAWN_DISTANCE, ATTACK_HITBOX_RADIUS, ATTACK_LIFETIME,
};
use crate::core::components::WorldPosition;
use crate::core::events::DamageEvent;
use crate::enemy::components::Enemy;
use crate::core::components::Facing;
use crate::player::components::Player;
use crate::projectile::components::{ArcHitbox, CircleHitbox, Lifetime, Projectile};

/// Point-blank radial swing (Space). Damage is resolved instantly this frame against every
/// enemy inside the circle; a transient VFX entity is spawned only so the gizmo can draw it.
pub fn player_circle_attack(
    mut commands: Commands,
    kb: Res<ButtonInput<KeyCode>>,
    mut damage_events: EventWriter<DamageEvent>,
    player: Query<(Entity, &WorldPosition, &Facing), With<Player>>,
    enemies: Query<(Entity, &WorldPosition), With<Enemy>>,
) {
    if !kb.just_pressed(KeyCode::Space) {
        return;
    }

    let Ok((player_entity, pos, facing)) = player.single() else {
        return;
    };

    // No aim direction yet (Facing starts at zero until the first mouse move).
    if facing.0.length_squared() < 1e-6 {
        return;
    }

    let center = pos.0 + facing.0 * ATTACK_SPAWN_DISTANCE;

    for (enemy_entity, enemy_pos) in &enemies {
        if center.distance(enemy_pos.0) <= ATTACK_HITBOX_RADIUS {
            damage_events.write(DamageEvent {
                target: enemy_entity,
                amount: CIRCLE_BASE_DMG,
                source: player_entity,
                tags: vec![],
            });
        }
    }

    commands.spawn((
        Projectile,
        WorldPosition(center),
        CircleHitbox { radius: ATTACK_HITBOX_RADIUS },
        Lifetime { timer: Timer::from_seconds(ATTACK_LIFETIME, TimerMode::Once) },
    ));
}

/// Cone swing (V) toward the aim direction. Same instant, multi-target resolution as the
/// circle attack, restricted to enemies within the arc's `half_angle`.
pub fn player_arc_attack(
    mut commands: Commands,
    kb: Res<ButtonInput<KeyCode>>,
    mut damage_events: EventWriter<DamageEvent>,
    player: Query<(Entity, &WorldPosition, &Facing), With<Player>>,
    enemies: Query<(Entity, &WorldPosition), With<Enemy>>,
) {
    if !kb.just_pressed(KeyCode::KeyV) {
        return;
    }

    let Ok((player_entity, pos, facing)) = player.single() else {
        return;
    };

    if facing.0.length_squared() < 1e-6 {
        return;
    }

    let forward = facing.0.normalize();
    let half_angle = std::f32::consts::FRAC_PI_4;
    let center = pos.0 + facing.0 * ATTACK_SPAWN_DISTANCE;

    for (enemy_entity, enemy_pos) in &enemies {
        let to_enemy = enemy_pos.0 - center;
        let dist = to_enemy.length();

        if dist > ATTACK_HITBOX_RADIUS {
            continue;
        }

        // An enemy exactly at the center has no direction; count it as inside the cone.
        let in_cone = dist < 1e-6 || forward.angle_to(to_enemy / dist).abs() <= half_angle;
        if in_cone {
            damage_events.write(DamageEvent {
                target: enemy_entity,
                amount: ARC_BASE_DMG,
                source: player_entity,
                tags: vec![],
            });
        }
    }

    commands.spawn((
        Projectile,
        WorldPosition(center),
        ArcHitbox { radius: ATTACK_HITBOX_RADIUS, half_angle },
        Facing(forward),
        Lifetime { timer: Timer::from_seconds(ATTACK_LIFETIME, TimerMode::Once) },
    ));
}
