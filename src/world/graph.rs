// Act graph — the branching encounter structure for one run.
//
// Structure: 3 acts × ~15 encounters each × 3 intertwining paths (Slay the Spire style).
// Generated at run start from RunRng (seed-deterministic).
//
// Node types:
//   Map       — fight room; has an objective type and a theme.
//   BossRoom  — themed room with a single boss enemy from ThemeDef.boss_pool.
//   ActBoss   — the act-ending boss fight; no theme (themed by act narrative, TBD).
//   ThroneRoom — kiss/curse room: mandatory curse modifier + rare talent reward on enter.
//               Always uses the "throne_room" layout generator (not the normal pool).
//   Merchant  — no combat; player can remove a talent or do a 3-for-1 trade-up.
//
// The player sees the encounter type and theme for reachable next nodes before choosing.
//
// ThroneRoom modifier:
//   Assigned at graph generation time from a pool of RoomModifierDefs.
//   The modifier is a set of StatModifiers that are applied as "extra_modifiers" to all
//   stat resolution calls during that encounter (see talent/modifier.rs resolve_params).
//
// Interactions:
//   - run/state.rs: ActGraph is stored in RunState and serialized.
//   - world/systems: reads current_node to load the right encounter.
//   - progression/systems/offer.rs: ThroneRoom nodes trigger a ThroneRoomRewardEvent.
//   - world/generator.rs: generates the room layout from the node's encounter type.

use crate::core::def_library::{DefAsset, DefLibrary};
use crate::enemy::assets::{EnemyId, ThemeId};
use crate::run::rng::RunRng;
use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;

pub type NodeId = u32;
pub type ModifierId = String;

/// Columns per act (Slay-the-Spire depth). Column 0 is the single entry node; the last column is a
/// single `ActBoss`; the second-to-last is a single `BossRoom`; the middle columns hold 1–3 nodes
/// each, one of which (in a RunRng-chosen middle column) is the guaranteed `ThroneRoom`.
pub const COLUMNS_PER_ACT: usize = 15;

/// ThroneRoom curse pool — ids match `assets/room_modifiers/<id>.roommod.ron`. One is RunRng-assigned
/// to each act's guaranteed ThroneRoom node at generation time (`EncounterNode.modifier`).
pub const THRONE_MODIFIERS: &[&str] = &["no_regen", "enemies_deal_double_damage", "player_slowed"];

/// The full graph for one act. Stored in RunState.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ActGraph {
    pub nodes: HashMap<NodeId, EncounterNode>,
    /// Directed edges: (from, to). A node may have multiple outgoing edges (branching). Sorted +
    /// deduped by `build_act_graph`, so `next_nodes` returns a stable, ascending order.
    pub edges: Vec<(NodeId, NodeId)>,
    pub entry: NodeId,
}

impl ActGraph {
    /// Returns all nodes reachable in one step from `from` (ascending NodeId — edges are sorted).
    pub fn next_nodes(&self, from: NodeId) -> Vec<NodeId> {
        self.edges.iter()
            .filter(|(f, _)| *f == from)
            .map(|(_, t)| *t)
            .collect()
    }

    /// The node with `id`, if present.
    pub fn node(&self, id: NodeId) -> Option<&EncounterNode> {
        self.nodes.get(&id)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EncounterNode {
    pub id: NodeId,
    /// Which column (0..COLUMNS_PER_ACT) this node sits in — feeds the depth/scaling driver (D5).
    pub column: usize,
    pub encounter: EncounterType,
    /// None for Merchant and ActBoss nodes.
    pub theme: Option<ThemeId>,
    /// Curse modifier applied during this encounter.
    /// Always Some for ThroneRoom; always None for everything else.
    pub modifier: Option<ModifierId>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EncounterType {
    /// Standard fight room. Objective type determines win condition.
    Map { objective: ObjectiveType },
    /// Themed room with one boss from ThemeDef.boss_pool.
    BossRoom,
    /// Act-ending boss fight.
    ActBoss,
    /// Kiss/curse room:
    ///   - Curse: mandatory StatModifier debuffs active for the entire fight (stored in modifier).
    ///   - Kiss:  pick 1 of 3 Rare-or-better talents before the fight begins.
    ///   - Layout: uses the "throne_room" layout generator (distinct geometry).
    ThroneRoom,
    /// Non-combat rest node. Player can remove a talent or 3-for-1 trade.
    Merchant,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ObjectiveType {
    Survive { duration_secs: f32 },
    KillAll,
    /// Kill the designated map boss. Boss is drawn from ThemeDef.map_boss_pool.
    KillMapBoss { boss_id: EnemyId },
}

/// Curse debuffs applied during a ThroneRoom encounter. Loaded from
/// assets/room_modifiers/<id>.roommod.ron. Uses the same `StatModifier` type as talents, threaded
/// into `resolve_params`'s `extra_modifiers` (Phase 7F) for Hostile casts (the curse makes the fight
/// harder). Ids match `THRONE_MODIFIERS`.
#[derive(Asset, TypePath, Debug, Clone, serde::Deserialize)]
pub struct RoomModifierDef {
    pub id: ModifierId,
    pub display_name: String,
    pub description: String, // shown to the player on entering the ThroneRoom
    /// Stat modifiers applied for the duration of the encounter.
    /// These stack with talent modifiers through the same resolve_params path.
    pub curse_modifiers: Vec<crate::talent::assets::StatModifier>,
}

/// Resource mapping ModifierId → Handle<RoomModifierDef>. Registered via
/// `register_def_library::<RoomModifierDef>()` in `RunPlugin`.
pub type RoomModifierLibrary = DefLibrary<RoomModifierDef>;

impl DefAsset for RoomModifierDef {
    // Compound `.roommod.ron` extension so the loader never collides with plain `.ron`.
    const EXTENSIONS: &'static [&'static str] = &["roommod.ron"];
    const MANIFEST: &'static [(&'static str, &'static str)] = &[
        ("no_regen", "room_modifiers/no_regen.roommod.ron"),
        ("enemies_deal_double_damage", "room_modifiers/enemies_deal_double_damage.roommod.ron"),
        ("player_slowed", "room_modifiers/player_slowed.roommod.ron"),
    ];
}

/// Builds one act's encounter graph. **Pure over `&mut RunRng`** — same seed ⇒ same graph, so it is
/// unit-testable for determinism + structural invariants and never touches `thread_rng` (the
/// reproducibility contract, docs/testing.md). Slay-the-Spire columns:
///   - column 0: a single entry `Map` (Act 1 → a KillAll **tutorial** map; Acts 2/3 → a random map);
///   - last column: a single `ActBoss`; second-to-last: a single `BossRoom`;
///   - middle columns: 1–3 nodes, mostly `Map` (random objective), the occasional `Merchant`, and
///     exactly one guaranteed `ThroneRoom` (with a RunRng-assigned curse);
///   - edges: each node links to 1–2 nodes in the next column, then a connectivity pass guarantees
///     every next-column node has ≥1 incoming edge (no dead ends; every node reachable from entry).
pub fn build_act_graph(act: u8, theme: ThemeId, rng: &mut RunRng) -> ActGraph {
    let mut nodes: HashMap<NodeId, EncounterNode> = HashMap::new();
    let mut columns: Vec<Vec<NodeId>> = Vec::with_capacity(COLUMNS_PER_ACT);
    let mut next_id: NodeId = 0;
    let alloc = |n: &mut NodeId| -> NodeId {
        let id = *n;
        *n += 1;
        id
    };

    // The guaranteed ThroneRoom lives in one middle column (1..=COLUMNS_PER_ACT-3).
    let throne_col = rng.rng().gen_range(1..(COLUMNS_PER_ACT - 2));

    for col in 0..COLUMNS_PER_ACT {
        let mut col_nodes: Vec<NodeId> = Vec::new();
        if col == 0 {
            // Act 1 opens on a fixed KillAll "tutorial" map (calibrated so the player reaches ~L2);
            // later acts start on a random-objective map, then branch.
            let objective = if act == 1 {
                ObjectiveType::KillAll
            } else {
                random_objective(rng)
            };
            let id = alloc(&mut next_id);
            nodes.insert(id, EncounterNode {
                id,
                column: col,
                encounter: EncounterType::Map { objective },
                theme: Some(theme.clone()),
                modifier: None,
            });
            col_nodes.push(id);
        } else if col == COLUMNS_PER_ACT - 1 {
            let id = alloc(&mut next_id);
            nodes.insert(id, EncounterNode {
                id,
                column: col,
                encounter: EncounterType::ActBoss,
                theme: None,
                modifier: None,
            });
            col_nodes.push(id);
        } else if col == COLUMNS_PER_ACT - 2 {
            let id = alloc(&mut next_id);
            nodes.insert(id, EncounterNode {
                id,
                column: col,
                encounter: EncounterType::BossRoom,
                theme: Some(theme.clone()),
                modifier: None,
            });
            col_nodes.push(id);
        } else {
            let count = rng.rng().gen_range(1..=3);
            for i in 0..count {
                let id = alloc(&mut next_id);
                let (encounter, node_theme, modifier) = if col == throne_col && i == 0 {
                    // The guaranteed ThroneRoom (first node of its column) — assign a curse.
                    let m = THRONE_MODIFIERS
                        [rng.rng().gen_range(0..THRONE_MODIFIERS.len())]
                        .to_string();
                    (EncounterType::ThroneRoom, Some(theme.clone()), Some(m))
                } else if rng.rng().gen_range(0..10) == 0 {
                    // ~10% of the remaining middle nodes are a no-combat Merchant (theme None).
                    (EncounterType::Merchant, None, None)
                } else {
                    (
                        EncounterType::Map { objective: random_objective(rng) },
                        Some(theme.clone()),
                        None,
                    )
                };
                nodes.insert(id, EncounterNode { id, column: col, encounter, theme: node_theme, modifier });
                col_nodes.push(id);
            }
        }
        columns.push(col_nodes);
    }

    // Edges. Each node in column c links to 1–2 nodes in column c+1; then a connectivity pass wires
    // any next-column node still lacking an incoming edge. Every node keeps ≥1 outgoing (the initial
    // loop) except the terminal ActBoss, and gains ≥1 incoming (the pass) except the entry.
    let mut edges: Vec<(NodeId, NodeId)> = Vec::new();
    for c in 0..(COLUMNS_PER_ACT - 1) {
        let cur = columns[c].clone();
        let nxt = columns[c + 1].clone();
        for &from in &cur {
            let k = rng.rng().gen_range(1..=nxt.len().min(2));
            for to in choose_k(&nxt, k, rng) {
                edges.push((from, to));
            }
        }
        for &to in &nxt {
            if !edges.iter().any(|(_, t)| *t == to) {
                let from = cur[rng.rng().gen_range(0..cur.len())];
                edges.push((from, to));
            }
        }
    }
    edges.sort_unstable();
    edges.dedup();

    let entry = columns[0][0];
    ActGraph { nodes, edges, entry }
}

/// A RunRng-picked objective for a middle/entry Map node. `KillMapBoss` names `warlord` (D4 — every
/// theme's map_boss_pool is `[warlord]` for now; completion tracks the `MapBoss` marker, not the id).
fn random_objective(rng: &mut RunRng) -> ObjectiveType {
    match rng.rng().gen_range(0..3) {
        0 => ObjectiveType::KillAll,
        1 => ObjectiveType::Survive { duration_secs: 30.0 },
        _ => ObjectiveType::KillMapBoss { boss_id: "warlord".to_string() },
    }
}

/// Picks up to `k` distinct nodes from `pool` using RunRng (seed-deterministic).
fn choose_k(pool: &[NodeId], k: usize, rng: &mut RunRng) -> Vec<NodeId> {
    let mut avail = pool.to_vec();
    let mut chosen = Vec::with_capacity(k);
    for _ in 0..k.min(avail.len()) {
        let idx = rng.rng().gen_range(0..avail.len());
        chosen.push(avail.remove(idx));
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn graph(seed: u64) -> ActGraph {
        build_act_graph(1, "sand_dune".to_string(), &mut RunRng::from_seed(seed))
    }

    #[test]
    fn same_seed_same_graph_different_seed_differs() {
        assert_eq!(graph(1), graph(1), "same seed ⇒ identical node + edge sets");
        assert_ne!(graph(1), graph(2), "different seeds ⇒ different graphs");
    }

    #[test]
    fn columns_entry_actboss_and_bossroom_invariants() {
        let g = graph(0xABCD);
        // Exactly COLUMNS_PER_ACT columns present.
        let cols: HashSet<usize> = g.nodes.values().map(|n| n.column).collect();
        assert_eq!(cols.len(), COLUMNS_PER_ACT);
        // Single entry node in column 0; it is the graph entry and a Map.
        let col0: Vec<&EncounterNode> = g.nodes.values().filter(|n| n.column == 0).collect();
        assert_eq!(col0.len(), 1, "single entry node");
        assert_eq!(col0[0].id, g.entry);
        assert!(matches!(col0[0].encounter, EncounterType::Map { .. }), "Act-1 entry is a Map (tutorial)");
        // Exactly one ActBoss, in the last column.
        let act_bosses: Vec<&EncounterNode> = g
            .nodes
            .values()
            .filter(|n| matches!(n.encounter, EncounterType::ActBoss))
            .collect();
        assert_eq!(act_bosses.len(), 1, "one act boss");
        assert_eq!(act_bosses[0].column, COLUMNS_PER_ACT - 1, "act boss is the last column");
        // Exactly one BossRoom, in the second-to-last column.
        let boss_rooms: Vec<&EncounterNode> = g
            .nodes
            .values()
            .filter(|n| matches!(n.encounter, EncounterType::BossRoom))
            .collect();
        assert_eq!(boss_rooms.len(), 1);
        assert_eq!(boss_rooms[0].column, COLUMNS_PER_ACT - 2);
    }

    #[test]
    fn at_least_one_throne_room_with_a_curse() {
        let g = graph(42);
        let thrones: Vec<&EncounterNode> = g
            .nodes
            .values()
            .filter(|n| matches!(n.encounter, EncounterType::ThroneRoom))
            .collect();
        assert!(!thrones.is_empty(), "every act has a ThroneRoom");
        for t in &thrones {
            let m = t.modifier.as_deref().expect("ThroneRoom always carries a curse");
            assert!(THRONE_MODIFIERS.contains(&m), "curse drawn from the pool");
        }
    }

    #[test]
    fn graph_is_connected_and_has_no_dead_ends() {
        let g = graph(7);
        // Every node reachable from entry (BFS over edges).
        let mut seen: HashSet<NodeId> = HashSet::new();
        let mut stack = vec![g.entry];
        while let Some(n) = stack.pop() {
            if !seen.insert(n) {
                continue;
            }
            for next in g.next_nodes(n) {
                stack.push(next);
            }
        }
        assert_eq!(seen.len(), g.nodes.len(), "every node reachable from entry");
        // No dead ends: every non-ActBoss node has ≥1 outgoing edge.
        for node in g.nodes.values() {
            if matches!(node.encounter, EncounterType::ActBoss) {
                continue;
            }
            assert!(
                !g.next_nodes(node.id).is_empty(),
                "node {} ({:?}) is a dead end",
                node.id,
                node.encounter
            );
        }
    }

    #[test]
    fn later_acts_open_on_a_random_objective_map() {
        // Act 2 entry is still a Map, but its objective is RunRng-picked (not forced KillAll).
        let g = build_act_graph(2, "forest".to_string(), &mut RunRng::from_seed(99));
        let entry = g.node(g.entry).unwrap();
        assert!(matches!(entry.encounter, EncounterType::Map { .. }));
        assert_eq!(entry.column, 0);
    }
}
