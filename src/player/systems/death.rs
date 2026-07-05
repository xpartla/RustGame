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
use crate::core::components::Health;
use crate::game::state::{GameOverSummary, GameState};
use crate::hero::components::HeroIdentity;
use crate::player::components::{Experience, Player};
use crate::run::state::RunState;

pub fn player_death(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    run_state: Option<Res<RunState>>,
    query: Query<(Entity, &Health, &Experience, &HeroIdentity), With<Player>>,
) {
    for (entity, health, exp, hero) in &query {
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
            commands.entity(entity).despawn();
            next_state.set(GameState::GameOver);
        }
    }
}
