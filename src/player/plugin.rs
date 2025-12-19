use std::collections::HashMap;
use bevy::prelude::*;
use crate::core::components::FlowField;
use crate::core::systems::flow_field::rebuild_flow_field_from_player;
use crate::player::systems::input::player_input;
use crate::enemy::systems::follow_flow_field::enemy_follow_flow_field;
use crate::player::systems::spawn_player::spawn_player;
use crate::player::systems::attack::player_melee_attack;
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FlowField {
            cost: HashMap::new(),
            direction: HashMap::new(),
        });
        app.add_systems(Startup, spawn_player);
        app.add_systems(
            Update,
            (
                player_input,
                player_melee_attack,
                rebuild_flow_field_from_player.after(player_input),
                enemy_follow_flow_field.after(rebuild_flow_field_from_player),
            ),
        );
    }
}