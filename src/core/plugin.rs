use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, in_state};
use crate::core::events::{AddGameplayEventExt, DamageEvent, GainXpEvent, HealEvent, LevelUpEvent};
use crate::core::sets::{CombatSet, MovementSet, StatusSet};
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
        // Damage/Heal resolve combat outcomes and can be written a frame before their reader
        // runs (DoT ticks) — they must survive overlay states. XP/LevelUp are consumed the same
        // frame they are written (gain_experience is ordered after CombatSet::Death), so the
        // standard two-frame expiry can never lose them.
        app.add_gameplay_event::<DamageEvent>();
        app.add_gameplay_event::<HealEvent>();
        app.add_event::<GainXpEvent>();
        app.add_event::<LevelUpEvent>();
        // The full frame skeleton: movement first (intent → integration), then the combat
        // resolution chain. Pinning movement ahead of combat keeps positions stable when later
        // phases add systems (see MovementSet docs in core/sets.rs).
        app.configure_sets(
            Update,
            (
                MovementSet::Intent,
                MovementSet::Integrate,
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
                            (apply_velocity, world_to_grid)
                                .chain()
                                .in_set(MovementSet::Integrate),
                            apply_damage.in_set(CombatSet::Apply),
                            apply_heal.in_set(CombatSet::Apply),
                            ).run_if(in_state(GameState::InRun)),
        );
    }
}
