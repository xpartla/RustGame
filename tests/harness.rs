// Golden scenario suite — harness smoke tests.
//
// Validates that the headless sim (rust_game::sim) boots the full game simulation without a
// window/GPU: Startup runs, the player exists, the map generates, RON assets load, and the
// state machine sits in InRun. Everything else in tests/ builds on this.

use rust_game::game::state::GameState;
use rust_game::sim::Sim;

#[test]
fn sim_boots_headless_into_in_run() {
    let mut sim = Sim::new(42);
    assert_eq!(sim.game_state(), GameState::InRun);
    assert!(sim.try_player().is_some(), "player spawned at startup");
    assert!(
        sim.world().resource::<rust_game::world::components::TileMap>().blocked.len() > 0,
        "map generated"
    );
}

#[test]
fn assets_load_and_level1_abilities_granted() {
    let mut sim = Sim::new_arena(42);
    let owned = sim.owned_abilities();
    assert!(owned.contains(&"death_strike".to_string()), "death_strike granted at L1");
    assert!(owned.contains(&"dnd".to_string()), "dnd granted at L1 (inert)");
    assert!(owned.contains(&"companion".to_string()), "companion granted at L1 (inert)");
}

#[test]
fn fixed_timestep_advances_simulated_time() {
    let mut sim = Sim::new_arena(42);
    let t0 = sim.world().resource::<bevy::time::Time>().elapsed_secs();
    sim.step(60);
    let t1 = sim.world().resource::<bevy::time::Time>().elapsed_secs();
    assert!(
        ((t1 - t0) - 1.0).abs() < 1e-3,
        "60 steps ≈ 1 simulated second, got {}",
        t1 - t0
    );
}
