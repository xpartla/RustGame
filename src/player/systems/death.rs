// Player death → the GameOver flow (Phase 7.5B).
//
// When the player's health reaches zero, capture a `GameOverSummary` (hero / level / act / node —
// read *before* the entity is despawned, since the run's `RunState` mirror can be encounter-stale),
// despawn the player so the world visibly empties, and transition to `GameState::GameOver`. That
// state freezes the InRun world (all gameplay systems are InRun-gated) and clears in-flight combat
// events (terminal-state clearing in core/events.rs). The death screen (`ui/screens/game_over.rs`)
// then offers R — restart / M — main menu via `run/systems/reset.rs`.
//
// Player-dependent systems (input, camera follow) already no-op when no player exists.
//
// Golden-master note: the campaign bot never dies, so the loop body never runs ⇒ byte-identical.

use bevy::prelude::*;
use crate::ability::components::AbilityInstance;
use crate::core::components::Health;
use crate::game::state::{GameOverSummary, GameState};
use crate::hero::components::HeroIdentity;
use crate::meta::state::MetaState;
use crate::player::components::{Experience, Player};
use crate::progression::state::LevelUpFlowState;
use crate::run::state::RunState;
use crate::run::systems::persistence::{record_run_end, sync_run_state};
use crate::talent::components::AcquiredTalents;

#[allow(clippy::too_many_arguments)]
pub fn player_death(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut run_state: Option<ResMut<RunState>>,
    mut meta: ResMut<MetaState>,
    level_flow: Res<LevelUpFlowState>,
    abilities: Query<(Entity, &AbilityInstance)>,
    query: Query<(Entity, &Health, &Experience, &HeroIdentity, &AcquiredTalents), With<Player>>,
) {
    for (entity, health, exp, hero, acquired) in &query {
        if health.current <= 0.0 {
            info!("Player died.");
            commands.insert_resource(GameOverSummary {
                victory: false,
                hero_id: hero.0.clone(),
                level: exp.level,
                act: run_state.as_deref().map(|r| r.current_act),
                node_column: run_state
                    .as_deref()
                    .and_then(|r| r.act_graph.node(r.current_node).map(|n| n.column)),
            });
            // Phase 8: sync the final build into RunState and record a scored RunRecord — a run
            // that is runless (a headless arena scenario has no RunState) has nothing to end.
            if let Some(run_state) = run_state.as_deref_mut() {
                sync_run_state(run_state, health.current, exp.level, entity, &abilities, acquired, &level_flow);
                record_run_end(&mut meta, run_state, false);
            }
            commands.entity(entity).despawn();
            next_state.set(GameState::GameOver);
        }
    }
}
