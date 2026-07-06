// Meta-state scenarios (Phase 8) — hero-unlock gating, run-end scoring, and the scoreboard's data
// source. Character-select/scoreboard are presentation-only (never run headless); these exercise
// their logic (run/systems/menu.rs, run/systems/persistence.rs::record_run_end) through the real
// input paths and the pure meta-state API. The golden campaign never starts a run or opens a menu,
// so none of this touches the golden master.

use bevy::prelude::KeyCode;
use rust_game::game::state::GameState;
use rust_game::meta::state::MetaState;
use rust_game::progression::state::LevelUpFlowState;
use rust_game::run::rng::RunRng;
use rust_game::run::state::RunState;
use rust_game::run::systems::persistence::record_run_end;
use rust_game::sim::Sim;
use rust_game::world::graph::build_act_graph;

/// A locked hero (Phase 8, §4 — the mechanism; D3 leaves every real hero unlocked) can't be picked:
/// character-select input refuses to emit a StartRunRequest, so no run starts.
#[test]
fn a_locked_hero_pick_is_refused() {
    let mut sim = Sim::new_arena(3);
    sim.lock_hero("mage");
    sim.enter_menu();
    sim.tap_key(KeyCode::Enter); // Menu -> CharacterSelect
    sim.step(1);
    assert_eq!(sim.game_state(), GameState::CharacterSelect);

    sim.select_hero_index(1); // the Mage is HeroDef::MANIFEST entry 1
    sim.step(2);

    assert_eq!(
        sim.game_state(),
        GameState::CharacterSelect,
        "a locked pick is refused — no StartRunRequest, no state change"
    );
    assert!(!sim.has_run());

    // The same digit still works for an unlocked hero (the refusal is pick-specific, not global).
    sim.select_hero_index(0); // blood_death_knight
    sim.step(9);
    assert_eq!(sim.game_state(), GameState::InRun);
    assert!(sim.has_run());
}

/// A defeat ends the run: it appends a scored `RunRecord` and clears the in-progress save (there is
/// nothing left to resume).
#[test]
fn run_end_appends_a_scored_run_record_on_defeat() {
    let mut sim = Sim::new_arena(11);
    sim.start_run(2222);
    sim.step(3);
    assert!(sim.meta().run_history.is_empty());

    let player = sim.player();
    sim.deal_damage(player, 1.0e6);
    sim.step(2);

    assert_eq!(sim.game_state(), GameState::GameOver);
    assert_eq!(sim.meta().run_history.len(), 1);
    let record = sim.meta().run_history[0].clone();
    assert_eq!(record.hero_id, "blood_death_knight");
    assert!(record.score > 0);
    assert!(sim.meta().in_progress_run.is_none(), "run end clears the save");
}

/// An Act-3 boss clear is a victory: same bookkeeping as a defeat, but the score carries the D2
/// victory bonus (meta::score's own unit tests cover the formula; this proves the live path calls it).
#[test]
fn run_end_appends_a_scored_run_record_on_victory() {
    use rust_game::world::graph::EncounterType;

    let mut sim = Sim::new_arena(13);
    sim.start_run(3333);
    sim.step(3);
    sim.world_mut().resource_mut::<RunState>().current_act = 3;
    sim.set_current_encounter(EncounterType::ActBoss, None, 29, None);
    sim.step(3);

    sim.kill_all_enemies();
    sim.step(3);

    assert_eq!(sim.game_over_victory(), Some(true));
    assert_eq!(sim.meta().run_history.len(), 1);
    let record = sim.meta().run_history[0].clone();
    assert!(record.act_reached >= 3);
    assert!(record.score > 0);
    assert!(sim.meta().in_progress_run.is_none());
}

fn synthetic_run(seed: u64, act: u8, level: u32) -> RunState {
    let mut rng = RunRng::from_seed(seed);
    let graph = build_act_graph(1, "sand_dune".to_string(), &mut rng);
    let entry = graph.entry;
    RunState {
        seed,
        hero_id: "blood_death_knight".to_string(),
        current_act: act,
        current_node: entry,
        act_graph: graph,
        player_health: 10.0,
        player_level: level,
        unlocked_abilities: Vec::new(),
        acquired_talents: Vec::new(),
        level_flow: LevelUpFlowState::new(Vec::new(), Vec::new()),
        elapsed_secs: 30.0,
    }
}

/// The scoreboard's data source: `run_history` accumulates one record per ended run, and sorting by
/// score (what `ui/screens/scoreboard.rs` does) puts the deepest/highest-scoring run first.
#[test]
fn scoreboard_data_sorts_run_history_by_score_descending() {
    let mut meta = MetaState::default();
    record_run_end(&mut meta, &synthetic_run(1, 1, 3), false);
    record_run_end(&mut meta, &synthetic_run(2, 3, 20), true);
    record_run_end(&mut meta, &synthetic_run(3, 2, 8), false);

    let mut sorted = meta.run_history.clone();
    sorted.sort_by(|a, b| b.score.cmp(&a.score));

    assert_eq!(sorted.len(), 3);
    assert!(sorted[0].score >= sorted[1].score && sorted[1].score >= sorted[2].score);
    assert_eq!(sorted[0].act_reached, 3, "the act-3 victory scores highest");
}
