// MetaState — account-level state that outlives any single run.
//
// Lives as a Resource inserted unconditionally at app startup (MetaPlugin::build), including in
// the headless sim — see meta/plugin.rs for the logic/disk split (Phase 8, §2 of the plan).
// Persists across GameState transitions, including game-over and return to menu.
// Contains NO run-specific data — that lives in RunState; the one exception is the *saved*
// snapshot of an interrupted run (`in_progress_run`), which is inert data until "Resume Run"
// hydrates it back into a live RunState (run/systems/persistence.rs::resume_run).
//
// Persistence: serialized to a local file by meta/persistence.rs.
// Format: serde (currently via RON; backend-swappable for future WASM/cloud save).
//
// Power does NOT persist between runs: no currency, no permanent stat upgrades.
// Only hero unlocks and scoreboard entries carry over.
//
// Interactions:
//   - meta/persistence.rs: load_meta_from_disk() at startup (windowed only), save_meta_to_disk()
//     whenever MetaState changes (windowed only).
//   - run/systems/persistence.rs: syncs + snapshots `in_progress_run` on every node transition;
//     appends a RunRecord and clears `in_progress_run` on run end (defeat or victory).
//   - ui/screens/character_select.rs: reads unlocked_heroes (via hero_is_unlocked) to grey out
//     locked heroes; run/systems/menu.rs refuses a locked pick.
//   - ui/screens/scoreboard.rs: reads run_history.

use crate::core::def_library::DefAsset;
use crate::hero::assets::{HeroDef, HeroId};
use crate::run::rng::RunRng;
use crate::run::state::RunState;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Inserted at app startup; never removed. Serialized to disk (windowed only).
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct MetaState {
    /// Heroes the player has unlocked. `Default` seeds every `HeroDef::MANIFEST` id (D3: "all
    /// unlocked, mechanism only" — concrete unlock triggers arrive with the real roster, Phase 9).
    pub unlocked_heroes: HashSet<HeroId>,
    /// Completed run records, newest first-appended (the scoreboard screen sorts by score).
    pub run_history: Vec<RunRecord>,
    /// If Some, there is a run in progress that "Resume Run" can hydrate. Cleared on run end
    /// (defeat or victory); refreshed at every node transition (run/systems/persistence.rs).
    pub in_progress_run: Option<SavedRun>,
}

/// First-launch / corrupt-save fallback (§2): every defined hero starts unlocked, matching D3 —
/// the greying *mechanism* exists and is tested, but nothing is actually locked yet.
impl Default for MetaState {
    fn default() -> Self {
        Self {
            unlocked_heroes: HeroDef::MANIFEST.iter().map(|(id, _)| id.to_string()).collect(),
            run_history: Vec::new(),
            in_progress_run: None,
        }
    }
}

/// Whether `hero_id` is currently unlocked. A pure predicate over `MetaState` so both the
/// character-select screen (greying) and its input handler (refusing a locked pick) agree.
pub fn hero_is_unlocked(meta: &MetaState, hero_id: &str) -> bool {
    meta.unlocked_heroes.contains(hero_id)
}

/// A no-op seam for Phase 9's concrete hero-unlock triggers (D3): called at run end so a future
/// "beat Act 2 as the Mage" style rule has a single, already-wired call site to fill in. Inert
/// today — every hero is already unlocked by default, so there is nothing to grant.
pub fn unlock_heroes_on_progress(_meta: &mut MetaState, _run: &RunState, _victory: bool) {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub hero_id: HeroId,
    /// Act reached before run ended (1–3; 3 = reached act 3 boss).
    pub act_reached: u8,
    pub score: u32,
    /// Unix timestamp (seconds since epoch) of run end. Real wall-clock time — never asserted in
    /// headless tests (the golden campaign never starts a run, so this path never runs there).
    pub timestamp_unix: u64,
}

/// The complete on-disk snapshot of an in-progress run: the run record + its RNG stream position,
/// bundled so resume is bit-exact (D1). Split back into the RunState + RunRng resources on load
/// (run/systems/persistence.rs::resume_run).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRun {
    pub run: RunState,
    pub rng: RunRng,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_unlocks_every_manifest_hero_and_has_no_history_or_save() {
        let meta = MetaState::default();
        assert_eq!(meta.unlocked_heroes.len(), HeroDef::MANIFEST.len());
        for (id, _) in HeroDef::MANIFEST {
            assert!(hero_is_unlocked(&meta, id), "{id} should be unlocked by default (D3)");
        }
        assert!(meta.run_history.is_empty());
        assert!(meta.in_progress_run.is_none());
    }

    #[test]
    fn hero_is_unlocked_respects_a_deliberately_locked_hero() {
        let mut meta = MetaState::default();
        meta.unlocked_heroes.remove("mage");
        assert!(!hero_is_unlocked(&meta, "mage"));
        assert!(hero_is_unlocked(&meta, "blood_death_knight"));
    }

    /// MetaState (incl. a populated `SavedRun` and `run_history`) round-trips through RON.
    #[test]
    fn meta_state_round_trips_through_ron() {
        use crate::progression::state::LevelUpFlowState;
        use crate::world::graph::build_act_graph;

        let mut rng = RunRng::from_seed(3);
        let graph = build_act_graph(1, "forest".to_string(), &mut rng);
        let entry = graph.entry;
        let run = RunState {
            seed: 3,
            hero_id: "mage".to_string(),
            current_act: 1,
            current_node: entry,
            act_graph: graph,
            player_health: 40.0,
            player_level: 2,
            unlocked_abilities: vec!["fireblast".to_string()],
            acquired_talents: vec![],
            level_flow: LevelUpFlowState::new(vec![], vec![]),
            elapsed_secs: 12.0,
        };

        let mut meta = MetaState::default();
        meta.run_history.push(RunRecord {
            hero_id: "blood_death_knight".to_string(),
            act_reached: 2,
            score: 4200,
            timestamp_unix: 1_720_000_000,
        });
        meta.in_progress_run = Some(SavedRun { run, rng });

        let ron = ron::ser::to_string(&meta).expect("serialize MetaState");
        let restored: MetaState = ron::de::from_str(&ron).expect("deserialize MetaState");

        assert_eq!(meta.unlocked_heroes, restored.unlocked_heroes);
        assert_eq!(meta.run_history, restored.run_history);
        assert_eq!(
            meta.in_progress_run.as_ref().map(|s| &s.run),
            restored.in_progress_run.as_ref().map(|s| &s.run)
        );
    }
}
