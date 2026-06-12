use bevy::color::palettes::css::GREEN;
use bevy::prelude::{Gizmos, Query, With};
use crate::core::components::WorldPosition;
use crate::core::components::Facing;
use crate::player::components::Player;

pub fn draw_player_facing(
    mut gizmos: Gizmos,
    player_q: Query<(&WorldPosition, &Facing), With<Player>>,
) {
    for (pos, facing) in &player_q {
        gizmos.line_2d(
            pos.0,
            pos.0 + facing.0 * 32.0,
            GREEN,
        );
    }
}
