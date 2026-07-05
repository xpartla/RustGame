use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Timer, TimerMode, Update, in_state};
use crate::core::components::FlowField;
use crate::core::def_library::DefLibraryAppExt;
use crate::core::sets::{CombatSet, MovementSet};
use crate::game::state::GameState;
use crate::core::systems::flow_field::rebuild_flow_field_from_player;
use crate::enemy::assets::EnemyDef;
use crate::enemy::components::EnemySpawner;
use crate::enemy::systems::follow_flow_field::enemy_follow_flow_field;
use crate::enemy::systems::ranged_caster::ranged_caster_ai;
use crate::enemy::systems::update_enemy_facing::update_enemy_facing;
use crate::enemy::systems::spawner::spawn_enemy_over_time;
use crate::enemy::systems::death::enemy_death;

// Presentation note: draw_enemy_attack_flash and enemy visuals (attach_enemy_visuals) are
// registered by game::presentation::PresentationPlugin.
//
// Contact melee is no longer a hardcoded system (Phase 5): enemies carry an auto-cast
// `contact_melee` ability that fires through ability/systems/execute.rs, so the old `enemy_attack`
// system and its AttackStats/AttackCooldown components are gone.

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<FlowField>()
            // EnemyDef asset + `.enemy.ron` loader + EnemyLibrary + Startup populate, in one call.
            .register_def_library::<EnemyDef>()
            .insert_resource(EnemySpawner {
                timer: Timer::from_seconds(5.0, TimerMode::Repeating),
                radius: 10,
            })
            .add_systems(Update, spawn_enemy_over_time.run_if(in_state(GameState::InRun)))
            .add_systems(Update, enemy_death.in_set(CombatSet::Death).run_if(in_state(GameState::InRun)))
            .add_systems(
                Update,
                (
                    rebuild_flow_field_from_player,
                    enemy_follow_flow_field,
                    ranged_caster_ai,
                    update_enemy_facing,
                )
                    .chain()
                    .in_set(MovementSet::Intent)
                    .run_if(in_state(GameState::InRun)),
            );
    }
}
