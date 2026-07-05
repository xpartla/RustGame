use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Timer, TimerMode, Update, in_state};
use crate::core::components::FlowField;
use crate::core::sets::{CombatSet, MovementSet};
use crate::game::state::GameState;
use crate::core::systems::flow_field::rebuild_flow_field_from_player;
use crate::enemy::components::EnemySpawner;
use crate::enemy::systems::follow_flow_field::enemy_follow_flow_field;
use crate::enemy::systems::update_enemy_facing::update_enemy_facing;
use crate::enemy::systems::spawner::spawn_enemy_over_time;
use crate::enemy::systems::death::enemy_death;
use crate::enemy::systems::attack::enemy_attack;

// Presentation note: draw_enemy_attack_flash and enemy visuals (attach_enemy_visuals) are
// registered by game::presentation::PresentationPlugin.

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<FlowField>()
            .insert_resource(EnemySpawner {
                timer: Timer::from_seconds(5.0, TimerMode::Repeating),
                radius: 10,
            })
            .add_systems(Update, spawn_enemy_over_time.run_if(in_state(GameState::InRun)))
            .add_systems(Update, enemy_attack.in_set(CombatSet::Damage).run_if(in_state(GameState::InRun)))
            .add_systems(Update, enemy_death.in_set(CombatSet::Death).run_if(in_state(GameState::InRun)))
            .add_systems(
                Update,
                (
                    rebuild_flow_field_from_player,
                    enemy_follow_flow_field,
                    update_enemy_facing,
                )
                    .chain()
                    .in_set(MovementSet::Intent)
                    .run_if(in_state(GameState::InRun)),
            );
    }
}
