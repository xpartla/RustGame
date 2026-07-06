// MapSelect — the minimal keyboard node picker (Phase 7, D3).
//
// After an encounter completes, `handle_encounter_complete` enters `GameState::MapSelect` (which
// freezes the InRun world, like the TalentPicker). This system reads 1/2/3 to pick among the reachable
// next nodes, tears down the cleared encounter, points RunState at the chosen node, and returns to
// InRun — where `load_encounter` builds the next room. The full visual act-graph map view is deferred
// to the UI phase (§8.1(9)); this mirrors the Phase-2 TalentPicker input handler and is sim-drivable
// headless (the presentation-only overlay lives in ui/screens/map_select.rs).

use bevy::prelude::*;

use crate::ability::components::AbilityInstance;
use crate::enemy::components::Enemy;
use crate::game::state::GameState;
use crate::pickup::components::PickUp;
use crate::projectile::components::Projectile;
use crate::run::state::{node_depth, CurrentEncounter, RoomModifiers, RunState};
use crate::run::systems::transitions::despawn_encounter_entities;
use crate::zone::components::PersistentZone;

/// Reads the player's branch pick (1/2/3) in `GameState::MapSelect` and loads the chosen node.
#[allow(clippy::too_many_arguments)]
pub fn handle_map_select(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut run_state: ResMut<RunState>,
    mut next_state: ResMut<NextState<GameState>>,
    mut room_mods: ResMut<RoomModifiers>,
    enemies: Query<Entity, With<Enemy>>,
    projectiles: Query<Entity, With<Projectile>>,
    zones: Query<Entity, With<PersistentZone>>,
    pickups: Query<Entity, With<PickUp>>,
    abilities: Query<(Entity, &AbilityInstance)>,
) {
    let selected = if keys.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else {
        None
    };
    let Some(idx) = selected else {
        return;
    };

    let reachable = run_state.act_graph.next_nodes(run_state.current_node);
    let Some(&chosen) = reachable.get(idx) else {
        return; // pressed a number past the offered branches — ignore
    };

    // Tear down the cleared encounter (the player entity persists) and clear any curse.
    despawn_encounter_entities(&mut commands, &enemies, &projectiles, &zones, &pickups, &abilities);
    room_mods.0.clear();

    run_state.current_node = chosen;
    let node = run_state
        .act_graph
        .node(chosen)
        .expect("chosen node exists in the graph")
        .clone();
    let depth = node_depth(run_state.current_act, node.column);
    // Overwrites the old CurrentEncounter (spawned = false) → load_encounter builds it in InRun.
    commands.insert_resource(CurrentEncounter::for_node(&node, depth));
    next_state.set(GameState::InRun);
}
