use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Timer, TimerMode, Update, in_state};
use crate::core::sets::CombatSet;
use crate::game::state::GameState;
use crate::pickup::components::PickUpSpawner;
use crate::pickup::constants::PICKUP_SPAWN_SECS;
use crate::pickup::systems::spawn_pickups::spawn_pickups_over_time;
use crate::pickup::systems::collect_pickups::collect_pickups;

pub struct PickUpPlugin;

impl Plugin for PickUpPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(PickUpSpawner {
                timer: Timer::from_seconds(PICKUP_SPAWN_SECS, TimerMode::Repeating),
            })
            .add_systems(Update, spawn_pickups_over_time.run_if(in_state(GameState::InRun)))
            .add_systems(Update, collect_pickups.in_set(CombatSet::Damage).run_if(in_state(GameState::InRun)));
    }
}
