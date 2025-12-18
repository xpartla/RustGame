use std::collections::{HashMap, VecDeque};
use bevy::prelude::{IVec2, Query, ResMut, Vec2, With};
use crate::constants::{FLOW_RADIUS};
use crate::core::components::{FlowField, GridPosition};
use crate::player::components::Player;

pub fn rebuild_flow_field_from_player(
    player_query: Query<&GridPosition, With<Player>>,
    mut flow_field: ResMut<FlowField>,
    // pass map info
) {
    let player_pos = match player_query.get_single() {
        Ok(pos) => *pos,
        Err(_) => return,
    };

    let mut cost_map = HashMap::new();
    let mut direction_map = HashMap::new();

    let mut queue = VecDeque::new();
    queue.push_back(player_pos);
    cost_map.insert(player_pos, 0);

    let neighbors = [IVec2::new(1,0), IVec2::new(-1,0), IVec2::new(0,1), IVec2::new(0,-1)];

    while let Some(current) = queue.pop_front() {
        let current_cost = cost_map[&current];

        for &offset in &neighbors {
            let neighbor = GridPosition { x: current.x + offset.x, y: current.y + offset.y };

            if cost_map.contains_key(&neighbor) {
                continue;
            }
            
            let dx = neighbor.x - player_pos.x;
            let dy = neighbor.y - player_pos.y;
            if(dx.abs() > FLOW_RADIUS || dy.abs() > FLOW_RADIUS) {
                continue;
            }
            // TODO: check obstacles 

            cost_map.insert(neighbor, current_cost + 1);
            direction_map.insert(neighbor, Vec2::new((current.x - neighbor.x) as f32, (current.y - neighbor.y) as f32));
            queue.push_back(neighbor);
        }
    }

    flow_field.cost = cost_map;
    flow_field.direction = direction_map;
}
