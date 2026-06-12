use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Timer, TimerMode, Update};
use crate::core::sets::CombatSet;
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
            .add_systems(Update, spawn_pickups_over_time)
            .add_systems(Update, collect_pickups.in_set(CombatSet::Damage));
    }
}
