// RunState — the single authoritative record of an in-progress run.
//
// Serialized on every node transition (encounter complete → next node selected).
// Written to MetaState.in_progress_run (as bytes). On "Resume Run", MetaState's
// saved blob is deserialized back into RunState and RunRng.
//
// Invariants:
//   - RunState is inserted as a Resource only during an active run.
//   - MetaState (meta/state.rs) is always present. They never share fields.
//   - RunRng is kept in a separate resource (run/rng.rs) so it can be passed as
//     ResMut<RunRng> independently, but it is serialized together with RunState.
//
// Interactions:
//   - run/systems/transitions.rs writes current_node and act on encounter completion.
//   - progression/state.rs LevelUpFlowState is stored inline here so it is saved.
//   - meta/persistence.rs serializes / deserializes this struct.

use crate::hero::assets::HeroId;
use crate::ability::assets::AbilityId;
use crate::talent::assets::TalentId;
use crate::world::graph::ActGraph;
use crate::progression::state::LevelUpFlowState;
use bevy::prelude::*;

/// The complete resumable run state. All fields are necessary and sufficient to
/// reconstruct the game state at the point the player left off.
#[derive(Resource, Debug, Clone)]
pub struct RunState {
    pub seed: u64,
    pub hero_id: HeroId,
    pub current_act: u8,        // 1, 2, or 3
    pub current_node: u32,      // NodeId in the ActGraph
    pub act_graph: ActGraph,
    pub player_health: f32,     // persisted across encounters (health is not restored between rooms)
    pub player_level: u32,
    pub unlocked_abilities: Vec<AbilityId>,
    pub acquired_talents: Vec<(TalentId, u8)>,
    pub level_flow: LevelUpFlowState,
}
