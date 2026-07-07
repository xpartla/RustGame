// Golden scenarios — Purgatory / cheat death (Phase 9.2).
//
// Locks in: a lethal hit is rescued to the resolved restore_percent of max health (purgatory.
// ability.ron: 5%, immunity_secs 5.0, cooldown 45.0), the rescued entity is immune to damage for
// immunity_secs, and — once immunity expires but the (much longer) cooldown hasn't — a second
// lethal hit is NOT rescued a second time.

use rust_game::game::state::GameState;
use rust_game::sim::Sim;

#[test]
fn purgatory_rescues_from_lethal_damage_and_grants_temporary_immunity() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    let player = sim.player();
    sim.grant_ability("purgatory");
    sim.step(1); // instance spawns

    sim.deal_damage(player, 10_000.0);
    sim.step(1);

    assert!(sim.try_player().is_some(), "the killing blow was intercepted, not fatal");
    assert_eq!(sim.game_state(), GameState::InRun, "no GameOver — Purgatory saved the run");
    assert!(
        (sim.player_health() - 10.0).abs() < 1e-3,
        "restored to 5% of 200 max hp = 10, got {}",
        sim.player_health()
    );

    // Immune for the next few seconds: even a huge hit does nothing.
    sim.deal_damage(player, 10_000.0);
    sim.step(1);
    assert!(
        (sim.player_health() - 10.0).abs() < 1e-3,
        "immunity window blocked the follow-up hit, got {}",
        sim.player_health()
    );
}

#[test]
fn a_second_lethal_hit_is_not_rescued_while_on_cooldown() {
    let mut sim = Sim::new_arena(43);
    sim.disable_companion();
    let player = sim.player();
    sim.grant_ability("purgatory");
    sim.step(1);

    sim.deal_damage(player, 10_000.0);
    sim.step(1);
    assert!(sim.try_player().is_some(), "first lethal hit rescued");

    // Past the 5.0s immunity window, but nowhere near the 45.0s cooldown.
    sim.step_seconds(5.1);
    sim.deal_damage(player, 10_000.0);
    sim.step(2);

    assert!(sim.try_player().is_none(), "cooldown gates a second rescue — this hit is fatal");
    assert_eq!(sim.game_state(), GameState::GameOver);
}
