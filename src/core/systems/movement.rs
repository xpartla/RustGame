use bevy::prelude::{Query, Res,};
use bevy::time::Time;
use crate::core::components::{Velocity, WorldPosition};

pub fn apply_velocity(
    mut query: Query<(&mut WorldPosition, &Velocity)>,
    time: Res<Time>,
) {
    for (mut pos, vel) in &mut query {
        pos.0 += vel.0 * time.delta_secs();
    }
}
