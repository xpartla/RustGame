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

use crate::enemy::assets::{EnemyId, ThemeId};
use std::collections::HashMap;

pub type NodeId = u32;
pub type ModifierId = String;

/// The full graph for one act. Stored in RunState.
#[derive(Debug, Clone)]
pub struct ActGraph {
    pub nodes: HashMap<NodeId, EncounterNode>,
    /// Directed edges: (from, to). A node may have multiple outgoing edges (branching).
    pub edges: Vec<(NodeId, NodeId)>,
    pub entry: NodeId,
}

impl ActGraph {
    /// Returns all nodes reachable in one step from `from`.
    pub fn next_nodes(&self, from: NodeId) -> Vec<NodeId> {
        self.edges.iter()
            .filter(|(f, _)| *f == from)
            .map(|(_, t)| *t)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct EncounterNode {
    pub id: NodeId,
    pub encounter: EncounterType,
    /// None for Merchant and ActBoss nodes.
    pub theme: Option<ThemeId>,
    /// Curse modifier applied during this encounter.
    /// Always Some for ThroneRoom; always None for everything else.
    pub modifier: Option<ModifierId>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum ObjectiveType {
    Survive { duration_secs: f32 },
    KillAll,
    /// Kill the designated map boss. Boss is drawn from ThemeDef.map_boss_pool.
    KillMapBoss { boss_id: EnemyId },
}

/// Curse debuffs applied during a ThroneRoom encounter.
/// Loaded from assets/room_modifiers/<id>.ron.
/// Uses the same StatModifier type as talents, passed as extra_modifiers to resolve_params.
#[derive(Debug, Clone)]
pub struct RoomModifierDef {
    pub id: ModifierId,
    pub display_name: String,
    pub description: String, // shown to the player on entering the ThroneRoom
    /// Stat modifiers applied to the player for the duration of the encounter.
    /// These stack with talent modifiers through the same resolve_params path.
    pub curse_modifiers: Vec<crate::talent::assets::StatModifier>,
}
