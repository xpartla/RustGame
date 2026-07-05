// RunState — the single authoritative record of an in-progress run — plus the small live-encounter
// state that gates the encounter systems.
//
// Phase 7 keeps these **in-memory** (no serde yet — that is Phase 8, §8.2). During a run the live
// player entity stays the source of truth for health/level/talents; RunState mirrors graph position
// + act and is synced on each encounter transition so Phase 8 can serialize it.
//
// Invariants:
//   - RunState / CurrentEncounter are inserted as Resources only during an active run. A runless
//     world (the golden campaign, which never calls start_run) has neither, so every encounter
//     system is `run_if`-gated off and the campaign is unaffected.
//   - MetaState (meta/state.rs) is always present. They never share fields.
//   - RunRng is kept in a separate resource (run/rng.rs) so it can be passed as ResMut<RunRng>
//     independently.
//
// Interactions:
//   - run/systems/transitions.rs writes current_node and act on encounter completion.
//   - progression/state.rs LevelUpFlowState is stored inline here so it is saved (Phase 8).
//   - world/graph.rs::build_act_graph builds `act_graph`.

use crate::ability::assets::AbilityId;
use crate::hero::assets::HeroId;
use crate::progression::state::LevelUpFlowState;
use crate::talent::assets::{StatModifier, TalentId};
use crate::world::graph::{ActGraph, EncounterNode, EncounterType, ModifierId, NodeId, ObjectiveType, COLUMNS_PER_ACT};
use bevy::prelude::*;

/// The complete resumable run state. All fields are necessary and sufficient to reconstruct the
/// game state at the point the player left off (Phase 8 serializes this).
#[derive(Resource, Debug, Clone)]
pub struct RunState {
    pub seed: u64,
    pub hero_id: HeroId,
    pub current_act: u8,   // 1, 2, or 3
    pub current_node: NodeId,
    pub act_graph: ActGraph,
    pub player_health: f32, // persisted across encounters (health is not restored between rooms)
    pub player_level: u32,
    pub unlocked_abilities: Vec<AbilityId>,
    pub acquired_talents: Vec<(TalentId, u8)>,
    pub level_flow: LevelUpFlowState,
}

/// The live "what am I fighting right now" state. Present only while an encounter is loaded; its
/// presence is the gate (`resource_exists::<CurrentEncounter>`) for every encounter system, so with
/// no run active the systems are inert.
#[derive(Resource, Debug)]
pub struct CurrentEncounter {
    pub node: NodeId,
    pub encounter: EncounterType,
    /// Theme id (for the roster's enemy pools); None for ActBoss / Merchant.
    pub theme: Option<String>,
    /// ThroneRoom curse id (None otherwise) — resolved to a RoomModifierDef by `load_encounter`.
    pub modifier: Option<ModifierId>,
    /// Scaling depth (D5) fed to `spawn_enemy_from_def` for this node.
    pub depth: u32,
    pub objective: ObjectiveProgress,
    /// Set true by `load_encounter` once the room + roster are generated (a one-shot guard).
    pub spawned: bool,
    /// A kill objective only completes after its targets have been *observed* present, so the
    /// same-frame gap between spawning the roster (Commands) and it appearing can't complete it early.
    pub armed: bool,
}

/// Per-objective tracking for the live encounter (mirrors `ObjectiveType`, plus a Merchant `Rest`).
#[derive(Debug)]
pub enum ObjectiveProgress {
    /// Complete once the roster has spawned and no `Enemy` remains.
    KillAll,
    /// Countdown; complete on expiry (enemies may persist).
    Survive { timer: Timer },
    /// Complete once the tagged `MapBoss` is dead (pack adds may remain).
    KillMapBoss,
    /// Merchant rest node — no combat; completes on load (ops deferred to Phase 8/9).
    Rest,
}

impl CurrentEncounter {
    /// Builds the live encounter for an `EncounterNode` at a scaling `depth`, deriving the objective
    /// from the encounter type. Starts un-spawned/un-armed; `load_encounter` fills the room + roster.
    pub fn for_node(node: &EncounterNode, depth: u32) -> Self {
        let objective = match &node.encounter {
            EncounterType::Map { objective } => match objective {
                ObjectiveType::KillAll => ObjectiveProgress::KillAll,
                ObjectiveType::Survive { duration_secs } => ObjectiveProgress::Survive {
                    timer: Timer::from_seconds(*duration_secs, TimerMode::Once),
                },
                ObjectiveType::KillMapBoss { .. } => ObjectiveProgress::KillMapBoss,
            },
            // Kill the boss (it is the only enemy in a boss room / act boss); clear the pack in a
            // throne room fight.
            EncounterType::BossRoom | EncounterType::ActBoss | EncounterType::ThroneRoom => {
                ObjectiveProgress::KillAll
            }
            EncounterType::Merchant => ObjectiveProgress::Rest,
        };
        Self {
            node: node.id,
            encounter: node.encounter.clone(),
            theme: node.theme.clone(),
            modifier: node.modifier.clone(),
            depth,
            objective,
            spawned: false,
            armed: false,
        }
    }
}

/// Active ThroneRoom curse modifiers (Phase 7F). Empty except during a ThroneRoom encounter; threaded
/// into `resolve_params`'s `extra_modifiers` for Hostile casts (the curse makes the fight harder).
/// Always present (default empty) so `execute_ready_abilities` can read it unconditionally — with an
/// empty list it is byte-identical to the pre-curse `&[]` path.
#[derive(Resource, Default, Debug)]
pub struct RoomModifiers(pub Vec<StatModifier>);

/// The scaling depth for a node (D5): a monotonic "how deep into the run" index. At the Act-1 tutorial
/// (act 1, column 0) depth is 0 ⇒ base enemy stats (matches Phase 5's neutral-at-depth-0 promise).
pub fn node_depth(act: u8, column: usize) -> u32 {
    ((act.max(1) as usize - 1) * COLUMNS_PER_ACT + column) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_formula_at_act_and_column_boundaries() {
        assert_eq!(node_depth(1, 0), 0, "Act-1 tutorial ⇒ base stats");
        assert_eq!(node_depth(1, 14), 14);
        assert_eq!(node_depth(2, 0), COLUMNS_PER_ACT as u32, "Act 2 entry continues the ramp");
        assert_eq!(node_depth(3, 0), (2 * COLUMNS_PER_ACT) as u32);
    }
}
