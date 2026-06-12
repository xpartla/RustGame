use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PostUpdate, Update};
use crate::core::events::DamageEvent;
use crate::core::sets::CombatSet;
use crate::core::systems::{
    movement::apply_velocity,
    render_sync::{apply_facing_rotation, sync_transform},
    grid_sync::world_to_grid,
    apply_damage::apply_damage,
    debug::draw_health_bars,
};

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app:&mut App) {
        app.add_event::<DamageEvent>();
        app.configure_sets(
            Update,
            (CombatSet::Damage, CombatSet::Apply, CombatSet::Death).chain(),
        );
        app.add_systems(Update,
                        (
                            apply_velocity,
                            world_to_grid.after(apply_velocity),
                            sync_transform.after(world_to_grid),
                            apply_facing_rotation.after(world_to_grid),
                            apply_damage.in_set(CombatSet::Apply),
                            ),
        );
        app.add_systems(PostUpdate, draw_health_bars);
    }
}