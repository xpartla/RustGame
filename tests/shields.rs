// Golden scenarios — shields / absorb (Phase 9.1, §8.1(5)).
//
// A generic damage-absorbing pool consumed before Health. No shipped content grants one yet (the
// first real consumer — bone shield — lands in Phase 9.2), so this test drives the primitive
// directly through `GainShieldEvent`, mirroring how tests/status.rs exercised statuses before any
// hero applied them.

use rust_game::sim::Sim;

#[test]
fn a_shielded_actor_takes_no_health_damage_until_the_pool_is_spent_then_spills() {
    let mut sim = Sim::new_arena(42);
    let enemy = sim.spawn_grunt((5, 0));
    let start_health = sim.enemy_health(enemy).unwrap();

    sim.give_shield(enemy, 10.0);
    sim.step(1); // apply_shield_gain

    sim.deal_damage(enemy, 6.0);
    sim.step(1);
    assert_eq!(sim.enemy_health(enemy), Some(start_health), "6 dmg fully absorbed");
    assert_eq!(sim.shield_amount(enemy), 4.0, "4 of the 10-pool shield remains");

    sim.deal_damage(enemy, 10.0);
    sim.step(1);
    // 4 remaining shield absorbs, 6 spills to health.
    assert_eq!(
        sim.enemy_health(enemy),
        Some(start_health - 6.0),
        "shield exhausted after 4, the other 6 spills to health"
    );
    assert_eq!(sim.shield_amount(enemy), 0.0, "shield fully drained and removed");
}

#[test]
fn shield_grants_stack_additively() {
    let mut sim = Sim::new_arena(42);
    let enemy = sim.spawn_grunt((5, 0));

    sim.give_shield(enemy, 5.0);
    sim.step(1);
    sim.give_shield(enemy, 5.0);
    sim.step(1);

    assert_eq!(sim.shield_amount(enemy), 10.0, "two grants stack into one pool");
}

#[test]
fn a_hit_smaller_than_the_pool_leaves_health_and_the_remainder_untouched() {
    let mut sim = Sim::new_arena(42);
    let enemy = sim.spawn_grunt((5, 0));
    let start_health = sim.enemy_health(enemy).unwrap();

    sim.give_shield(enemy, 100.0);
    sim.step(1);

    sim.deal_damage(enemy, 25.0);
    sim.step(1);

    assert_eq!(sim.enemy_health(enemy), Some(start_health), "no health lost under a large shield");
    assert_eq!(sim.shield_amount(enemy), 75.0);
}
