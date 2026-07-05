// Act-graph generation scenarios (Phase 7) — the seed-determinism + structural invariants of
// `build_act_graph`, exercised through the public crate API (docs/testing.md Phase-7 DoD:
// "act graph is seed-deterministic").

use std::collections::HashSet;

use rust_game::run::rng::RunRng;
use rust_game::world::graph::{build_act_graph, ActGraph, EncounterType, NodeId, COLUMNS_PER_ACT};

fn graph(act: u8, theme: &str, seed: u64) -> ActGraph {
    build_act_graph(act, theme.to_string(), &mut RunRng::from_seed(seed))
}

#[test]
fn act_graph_is_seed_deterministic() {
    // Same seed ⇒ identical node + edge sets; a different seed ⇒ a different graph.
    assert_eq!(graph(1, "sand_dune", 0x5EED), graph(1, "sand_dune", 0x5EED));
    assert_ne!(graph(1, "sand_dune", 0x5EED), graph(1, "sand_dune", 0x5EEE));
}

#[test]
fn graph_is_connected_with_one_act_boss() {
    let g = graph(1, "forest", 12345);

    // Exactly one ActBoss, in the last column.
    let act_bosses: Vec<_> = g
        .nodes
        .values()
        .filter(|n| matches!(n.encounter, EncounterType::ActBoss))
        .collect();
    assert_eq!(act_bosses.len(), 1, "one act boss per act");
    assert_eq!(act_bosses[0].column, COLUMNS_PER_ACT - 1);

    // At least one ThroneRoom.
    assert!(
        g.nodes.values().any(|n| matches!(n.encounter, EncounterType::ThroneRoom)),
        "every act has a ThroneRoom"
    );

    // Every node reachable from entry (BFS), and no non-boss dead ends.
    let mut seen: HashSet<NodeId> = HashSet::new();
    let mut stack = vec![g.entry];
    while let Some(n) = stack.pop() {
        if seen.insert(n) {
            stack.extend(g.next_nodes(n));
        }
    }
    assert_eq!(seen.len(), g.nodes.len(), "connected front-to-back");
    for node in g.nodes.values() {
        if !matches!(node.encounter, EncounterType::ActBoss) {
            assert!(!g.next_nodes(node.id).is_empty(), "no dead ends");
        }
    }
}
