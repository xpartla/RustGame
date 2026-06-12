use bevy::app::PostUpdate;
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Timer, TimerMode, Update};
use crate::core::components::FlowField;
use crate::core::sets::CombatSet;
use crate::core::systems::flow_field::rebuild_flow_field_from_player;
use crate::enemy::components::EnemySpawner;
use crate::enemy::systems::debug::draw_enemy_attack_flash;
use crate::enemy::systems::follow_flow_field::enemy_follow_flow_field;
use crate::enemy::systems::update_enemy_facing::update_enemy_facing;
use crate::enemy::systems::spawner::spawn_enemy_over_time;
use crate::enemy::systems::death::enemy_death;
use crate::enemy::systems::attack::enemy_attack;
pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<FlowField>()
            .insert_resource(EnemySpawner {
                timer: Timer::from_seconds(5.0, TimerMode::Repeating),
                radius: 10,
            })
            .add_systems(Update, spawn_enemy_over_time)
            .add_systems(Update, enemy_attack.in_set(CombatSet::Damage))
            .add_systems(Update, enemy_death.in_set(CombatSet::Death))
            .add_systems(
                Update,
                (
                    rebuild_flow_field_from_player,
                    enemy_follow_flow_field.after(rebuild_flow_field_from_player),
                    update_enemy_facing.after(enemy_follow_flow_field),
                ),
            )
            .add_systems(PostUpdate, draw_enemy_attack_flash);
    }
}
