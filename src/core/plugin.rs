use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, in_state};
use crate::core::events::{DamageEvent, GainXpEvent, HealEvent, LevelUpEvent};
use crate::core::sets::{CombatSet, StatusSet};
use crate::game::state::GameState;
use crate::core::systems::{
    movement::apply_velocity,
    grid_sync::world_to_grid,
    apply_damage::apply_damage,
    apply_heal::apply_heal,
};

// Presentation note: sync_transform / apply_facing_rotation / draw_health_bars are visual
// consumers of core state and are registered by game::presentation::PresentationPlugin, so
// the simulation stays headless-safe. Their ordering relative to world_to_grid is preserved
// there.

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app:&mut App) {
        app.add_event::<DamageEvent>();
        app.add_event::<HealEvent>();
        app.add_event::<GainXpEvent>();
        app.add_event::<LevelUpEvent>();
        app.configure_sets(
            Update,
            (
                CombatSet::Damage,
                CombatSet::Apply,
                StatusSet::Tick,
                StatusSet::CrossInteract,
                CombatSet::Death,
            )
                .chain(),
        );
        app.add_systems(Update,
                        (
                            apply_velocity,
                            world_to_grid.after(apply_velocity),
                            apply_damage.in_set(CombatSet::Apply),
                            apply_heal.in_set(CombatSet::Apply),
                            ).run_if(in_state(GameState::InRun)),
        );
    }
}
