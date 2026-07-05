// Act-graph map view (Phase 7.5D) — the branch picker shown in `GameState::MapSelect`.
//
// Upgrades Phase 7's flat text list into a Slay-the-Spire column view of `RunState.act_graph`: nodes
// laid out by their `column`, encounter-type glyph + label, theme, with the current node highlighted,
// past columns dimmed, and the reachable next nodes numbered to match the selection keys. The input
// contract is unchanged — run/systems/select.rs::handle_map_select still reads 1/2/3 — so this is a
// pure presentation change (golden-master-neutral by construction; never runs headless).
//
// The player sees the encounter type + theme (+ objective) for reachable nodes before choosing
// (Mechanics: "the player can see the encounter type, and the map theme").

use bevy::prelude::*;

use crate::run::state::RunState;
use crate::ui::theme::{self, text};
use crate::world::graph::{ActGraph, EncounterType, NodeId, ObjectiveType, COLUMNS_PER_ACT};

/// Root overlay node. Despawned on exit (recursively removes the whole subtree).
#[derive(Component)]
pub struct MapSelectRoot;

/// Spawns the act-graph view on entering MapSelect.
pub fn spawn_map_select(mut commands: Commands, run_state: Option<Res<RunState>>) {
    let Some(run_state) = run_state else {
        return;
    };
    let graph = &run_state.act_graph;
    let current = run_state.current_node;
    let current_col = graph.node(current).map(|n| n.column).unwrap_or(0);
    let reachable = graph.next_nodes(current);

    commands
        .spawn((MapSelectRoot, theme::overlay_root(), BackgroundColor(theme::OVERLAY_BG)))
        .with_children(|root| {
            root.spawn(text(
                format!("ACT {}  —  Choose your path", run_state.current_act),
                theme::FS_TITLE,
                theme::TITLE,
            ));

            // Columns laid out left → right; each column is a vertical stack of its node cells.
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|columns| {
                for col in 0..COLUMNS_PER_ACT {
                    let mut ids: Vec<NodeId> = graph
                        .nodes
                        .iter()
                        .filter(|(_, n)| n.column == col)
                        .map(|(id, _)| *id)
                        .collect();
                    ids.sort_unstable(); // stable, deterministic column order

                    columns
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|column| {
                            for id in ids {
                                let reach_idx = reachable.iter().position(|r| *r == id);
                                let (label, color) =
                                    node_cell(graph, id, id == current, reach_idx, col < current_col);
                                column.spawn(text(label, theme::FS_SMALL, color));
                            }
                        });
                }
            });

            root.spawn(text(
                "1 / 2 / 3 to choose a highlighted branch",
                theme::FS_HINT,
                theme::HINT,
            ));
        });
}

/// The label + color for one node cell, given its role: current (highlighted), reachable (numbered +
/// accented, with its objective/theme shown), a past column (dimmed), or a not-yet-reachable node.
fn node_cell(
    graph: &ActGraph,
    id: NodeId,
    is_current: bool,
    reachable_idx: Option<usize>,
    is_past: bool,
) -> (String, Color) {
    let Some(node) = graph.node(id) else {
        return ("?".to_string(), theme::DIM);
    };
    let glyph = glyph(&node.encounter);

    if is_current {
        return (format!("[{glyph}]* HERE"), theme::ACCENT);
    }
    if let Some(i) = reachable_idx {
        let detail = describe(&node.encounter, node.theme.as_deref());
        return (format!("{}. [{glyph}] {detail}", i + 1), theme::TITLE);
    }
    let color = if is_past { theme::DIM } else { theme::TEXT };
    (format!("[{glyph}]"), color)
}

/// A single-character glyph for an encounter type (text only — no art assets this phase).
fn glyph(encounter: &EncounterType) -> char {
    match encounter {
        EncounterType::Map { .. } => 'M',
        EncounterType::BossRoom => 'B',
        EncounterType::ActBoss => 'A',
        EncounterType::ThroneRoom => 'T',
        EncounterType::Merchant => '$',
    }
}

/// A short human label: encounter type (+ objective for Map nodes) and theme.
fn describe(encounter: &EncounterType, theme: Option<&str>) -> String {
    let kind = match encounter {
        EncounterType::Map { objective } => match objective {
            ObjectiveType::KillAll => "Fight".to_string(),
            ObjectiveType::Survive { duration_secs } => format!("Survive {duration_secs:.0}s"),
            ObjectiveType::KillMapBoss { .. } => "Mini-boss".to_string(),
        },
        EncounterType::BossRoom => "Boss Room".to_string(),
        EncounterType::ActBoss => "Act Boss".to_string(),
        EncounterType::ThroneRoom => "Throne Room".to_string(),
        EncounterType::Merchant => "Merchant".to_string(),
    };
    match theme {
        Some(t) => format!("{kind} · {t}"),
        None => kind,
    }
}

/// Tears the overlay down on leaving MapSelect.
pub fn despawn_map_select(mut commands: Commands, root: Query<Entity, With<MapSelectRoot>>) {
    for entity in &root {
        commands.entity(entity).despawn();
    }
}
