use bevy::prelude::{Entity, EventWriter, Query, Res, Time, With};
use crate::core::components::WorldPosition;
use crate::core::events::DamageEvent;
use crate::enemy::components::{AttackCooldown, AttackStats, Enemy};
use crate::player::components::Player;

/// Contact attack: while an enemy is within `ENEMY_ATTACK_RANGE` of the player and its
/// cooldown is ready, it emits a `DamageEvent` at the player and resets the cooldown.
/// Runs in `CombatSet::Damage` so the hit resolves the same frame (see core/sets.rs).
pub fn enemy_attack(
    time: Res<Time>,
    mut damage_events: EventWriter<DamageEvent>,
    player: Query<(Entity, &WorldPosition), With<Player>>,
    mut enemies: Query<(Entity, &WorldPosition, &mut AttackCooldown, &AttackStats), With<Enemy>>,
) {
    let Ok((player_entity, player_pos)) = player.single() else {
        return;
    };

    for (enemy_entity, enemy_pos, mut cooldown, stats) in &mut enemies {
        cooldown.timer.tick(time.delta());

        if enemy_pos.0.distance(player_pos.0) > stats.range {
            continue;
        }

        if cooldown.timer.finished() {
            damage_events.write(DamageEvent {
                target: player_entity,
                amount: stats.damage,
                source: enemy_entity,
            });
            cooldown.timer.reset();
        }
    }
}
