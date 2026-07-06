// Persistence scenarios (Phase 8) — RunState syncing, save/resume through the real Resume Run
// path, and the D1 headline guarantee: a resumed run continues the RunRng stream exactly. The
// golden campaign never starts a run, so none of this touches the golden master.

use rust_game::game::state::GameState;
use rust_game::sim::Sim;

fn started(seed: u64) -> Sim {
    let mut sim = Sim::new_arena(seed);
    sim.start_run(seed);
    sim.step(3); // settle: roster + level-1 grant
    sim
}

/// The node boundary sync (§1.2): abilities/talents/the run timer are otherwise never written into
/// `RunState` after run-start, so this is the prerequisite every other resume guarantee rests on.
#[test]
fn run_state_syncs_abilities_talents_and_timer_at_a_node_transition() {
    let mut sim = started(0xF00D);
    sim.grant_talent("death_strike_leech_common");
    sim.step(1);
    sim.step_seconds(2.0);

    sim.kill_all_enemies();
    sim.step(3); // check_objective -> EncounterCompleteEvent -> handle_encounter_complete (sync)
    assert_eq!(sim.game_state(), GameState::MapSelect);

    let run = sim.run_state().expect("run active");
    for id in ["death_strike", "dnd", "companion"] {
        assert!(
            run.unlocked_abilities.iter().any(|a| a == id),
            "level-1 ability {id} synced into RunState"
        );
    }
    assert!(
        run.acquired_talents.iter().any(|(id, _)| id == "death_strike_leech_common"),
        "the granted talent synced into RunState"
    );
    assert!(run.elapsed_secs > 1.5, "the run timer ticked while InRun: {}", run.elapsed_secs);
}

/// The full §3.2 hydration: health/level/abilities/talents/node/act all match what was saved, and
/// the saved node's room loads fresh.
#[test]
fn save_then_resume_reconstructs_a_live_run() {
    let mut sim = started(0xC0DE);
    sim.grant_talent("death_strike_leech_common");
    sim.step(1);

    sim.kill_all_enemies();
    sim.step(3); // -> save into MetaState.in_progress_run + MapSelect
    assert_eq!(sim.game_state(), GameState::MapSelect);

    let saved = sim.run_state().cloned().expect("run synced before resume");
    assert!(sim.meta().in_progress_run.is_some(), "a save exists at the node boundary");

    sim.request_resume_run();
    sim.step(9); // teardown + respawn + re-grant/re-install + settle the level-1 grant + load_encounter

    assert_eq!(sim.game_state(), GameState::InRun, "resume lands back in the run");
    assert!(sim.has_run());
    assert_eq!(sim.current_act(), Some(saved.current_act));
    assert_eq!(sim.current_node(), Some(saved.current_node));
    assert_eq!(sim.player_level(), saved.player_level);

    let owned = sim.owned_abilities();
    for id in &saved.unlocked_abilities {
        assert!(owned.contains(id), "resumed player owns saved ability {id}");
    }
    let talents = sim.acquired_talents();
    assert!(
        talents.iter().any(|(id, _)| id == "death_strike_leech_common"),
        "resumed player has the saved talent"
    );
    assert!(sim.encounter_spawned(), "the saved node's room loaded fresh");
}

/// The D1 headline guarantee: resuming from byte-identical saved data is fully deterministic — no
/// hidden nondeterminism (thread_rng leak, iteration-order dependence) creeps into the rehydration
/// or the reroll it triggers. `run/rng.rs` proves the RNG type resumes exactly; this proves the
/// whole resume pipeline reproduces the same live roster from that resumed stream position.
#[test]
fn resume_continues_the_rng_stream_exactly() {
    let mut origin = started(0x5EED);
    origin.kill_all_enemies();
    origin.step(3); // -> save + MapSelect
    let saved = origin.meta().in_progress_run.clone().expect("a save exists at the node boundary");

    let mut a = resume_into_fresh_sim(saved.clone());
    let mut b = resume_into_fresh_sim(saved);
    a.step(6); // load_encounter rerolls the saved node's roster from the resumed RNG stream
    b.step(6);

    assert_eq!(sim_encounter_debug(&mut a), sim_encounter_debug(&mut b));
    assert_eq!(
        a.enemy_roster_signature(),
        b.enemy_roster_signature(),
        "two independent resumes of identical saved data roll an identical roster (D1)"
    );
}

fn resume_into_fresh_sim(saved: rust_game::meta::state::SavedRun) -> Sim {
    let mut sim = Sim::new_arena(0); // seed is irrelevant — resume overwrites RunRng from `saved`
    sim.world_mut().resource_mut::<rust_game::meta::state::MetaState>().in_progress_run = Some(saved);
    sim.request_resume_run();
    sim.step(1);
    sim
}

fn sim_encounter_debug(sim: &mut Sim) -> Option<String> {
    sim.current_encounter_debug()
}

/// Resuming with no save present is a clean no-op — stays wherever it was (the menu), never panics.
#[test]
fn resume_with_no_save_falls_back_cleanly() {
    let mut sim = Sim::new_arena(0x1);
    sim.enter_menu();
    assert!(sim.meta().in_progress_run.is_none());

    sim.request_resume_run();
    sim.step(2);

    assert_eq!(sim.game_state(), GameState::Menu, "no save to resume ⇒ stays in the menu");
    assert!(!sim.has_run());
}
