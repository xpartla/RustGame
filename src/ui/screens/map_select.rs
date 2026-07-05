// Map-select overlay — the minimal branch picker shown in GameState::MapSelect (Phase 7).
//
// Display only. It reads RunState (the act graph + current node) to list the reachable next nodes with
// their encounter type + theme; input (1/2/3) is handled by run/systems/select.rs::handle_map_select,
// which mutates RunState and loads the chosen node. The full visual act-graph map view is deferred to
// the UI phase; this mirrors the Phase-2 TalentPicker overlay. Never runs headless (presentation only).

use bevy::prelude::*;

use crate::run::state::RunState;
use crate::world::graph::EncounterType;

/// Root overlay node. Despawned on exit (recursively removes the whole subtree).
#[derive(Component)]
pub struct MapSelectRoot;

/// Spawns the branch list on entering MapSelect.
pub fn spawn_map_select(mut commands: Commands, run_state: Option<Res<RunState>>) {
    let Some(run_state) = run_state else {
        return;
    };
    let reachable = run_state.act_graph.next_nodes(run_state.current_node);

    commands
        .spawn((
            MapSelectRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.05, 0.82)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!("ACT {}  —  Choose your path", run_state.current_act)),
                TextFont { font_size: 40.0, ..default() },
                TextColor(Color::srgb(0.85, 0.9, 0.98)),
            ));
            for (i, node_id) in reachable.iter().enumerate() {
                let label = run_state
                    .act_graph
                    .node(*node_id)
                    .map(|n| describe(&n.encounter, n.theme.as_deref()))
                    .unwrap_or_else(|| "?".to_string());
                root.spawn((
                    Text::new(format!("{}.   {}", i + 1, label)),
                    TextFont { font_size: 28.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.9, 0.95)),
                ));
            }
            root.spawn((
                Text::new("1 / 2 / 3 to choose a branch"),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.7)),
            ));
        });
}

fn describe(encounter: &EncounterType, theme: Option<&str>) -> String {
    let kind = match encounter {
        EncounterType::Map { objective } => format!("Map ({objective:?})"),
        EncounterType::BossRoom => "Boss Room".to_string(),
        EncounterType::ActBoss => "Act Boss".to_string(),
        EncounterType::ThroneRoom => "Throne Room".to_string(),
        EncounterType::Merchant => "Merchant".to_string(),
    };
    match theme {
        Some(t) => format!("{kind}  —  {t}"),
        None => kind,
    }
}

/// Tears the overlay down on leaving MapSelect.
pub fn despawn_map_select(mut commands: Commands, root: Query<Entity, With<MapSelectRoot>>) {
    for entity in &root {
        commands.entity(entity).despawn();
    }
}
