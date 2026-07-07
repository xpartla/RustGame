// Golden scenarios — the remaining BDK class passives (Phase 9.2):
// bdk_passive_no_heal_cap, bdk_passive_overkill_leech, bdk_passive_health_and_healing.

use bevy::math::Vec2;
use rust_game::core::components::HealingTakenModifier;
use rust_game::sim::Sim;

#[test]
fn no_heal_cap_clamps_health_to_35_percent_of_max() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.grant_talent("bdk_passive_no_heal_cap");
    sim.step(1);

    // DK max is 200 (Phase 9.2 base_stats); 35% = 70. Set well above the cap.
    sim.set_player_health(150.0);
    sim.step(1);

    assert!(
        (sim.player_health() - 70.0).abs() < 1e-3,
        "clamped to 35% of 200 max = 70, got {}",
        sim.player_health()
    );
}

#[test]
fn no_heal_cap_boosts_leech_by_1_5x() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    sim.grant_talent("bdk_passive_no_heal_cap");
    sim.step(1);

    // Keep the player well under the 70-hp cap so the clamp never interferes with reading the
    // leech delta directly.
    sim.set_player_health(20.0);

    let enemy = sim.spawn_grunt((1, 0));
    sim.set_health(enemy, 1000.0); // durable dummy — no kill, no overkill-leech interference
    sim.trigger_ability("death_strike");
    sim.step(2); // damage + leech resolve in the same combat frame; +1 for the heal to land

    // 10 base damage * 5% leech_percent * 1.5x boost = 0.75 healed.
    let healed = sim.player_health() - 20.0;
    assert!((healed - 0.75).abs() < 1e-3, "expected +0.75 (5% * 1.5x of 10 dmg), got {healed}");
}

#[test]
fn overkill_leech_heals_for_20_percent_of_the_overkill_amount() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    sim.grant_talent("bdk_passive_overkill_leech");
    sim.set_ability_param("death_strike", "leech_percent", 0.0); // isolate overkill-leech only
    sim.step(1);

    sim.set_player_health(20.0);
    let enemy = sim.spawn_grunt((1, 0));
    sim.set_health(enemy, 1.0); // 10 dmg vs 1 hp -> 9 overkill
    sim.trigger_ability("death_strike");
    sim.step(2);

    let healed = sim.player_health() - 20.0;
    assert!((healed - 1.8).abs() < 1e-3, "expected +1.8 (20% of 9 overkill), got {healed}");
}

#[test]
fn no_overkill_leech_talent_means_no_heal_from_a_kill() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    sim.set_ability_param("death_strike", "leech_percent", 0.0);
    sim.step(1);

    sim.set_player_health(20.0);
    let enemy = sim.spawn_grunt((1, 0));
    sim.set_health(enemy, 1.0);
    sim.trigger_ability("death_strike");
    sim.step(2);

    assert_eq!(sim.player_health(), 20.0, "no talent, no overkill-leech heal");
}

#[test]
fn health_and_healing_scales_max_hp_and_healing_taken_per_stack() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();

    sim.grant_talent("bdk_passive_health_and_healing");
    sim.step(1);
    let player = sim.player();
    assert!(
        (sim.world().get::<rust_game::core::components::Health>(player).unwrap().max - 220.0).abs() < 1e-3,
        "1 stack: 200 * 1.10 = 220"
    );
    assert_eq!(
        sim.world().get::<HealingTakenModifier>(player).map(|m| m.0),
        Some(1.15),
        "1 stack: +15% healing taken"
    );

    sim.grant_talent("bdk_passive_health_and_healing");
    sim.grant_talent("bdk_passive_health_and_healing");
    sim.step(1);
    assert!(
        (sim.world().get::<rust_game::core::components::Health>(player).unwrap().max - 260.0).abs() < 1e-3,
        "3 stacks: 200 * 1.30 = 260"
    );
    assert_eq!(
        sim.world().get::<HealingTakenModifier>(player).map(|m| m.0),
        Some(1.45),
        "3 stacks: +45% healing taken"
    );
}
