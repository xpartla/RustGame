use bevy::prelude::{Query, Vec2, With};
use crate::constants::ENEMY_SPEED;
use crate::core::components::{GridPosition, Velocity};
use crate::enemy::components::Enemy;
use crate::player::components::Player;

pub fn chase_player(
    player: Query<&GridPosition, With<Player>>,
    mut enemies: Query<(&GridPosition, &mut Velocity), With<Enemy>>,
) {
    let player_pos = player.single().unwrap();

    for (enemy_pos, mut vel) in &mut enemies {
        let dir = Vec2::new(
            (player_pos.x - enemy_pos.x) as f32,
            (player_pos.y - enemy_pos.y) as f32,
        ).normalize_or_zero();

        vel.0 = dir * ENEMY_SPEED;
    }
}
