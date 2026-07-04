// Golden scenarios — combat.
//
// Locks in: Death Strike's cone membership / damage / leech / cooldown (from
// death_strike.ability.ron: damage 10, range 60, half_angle 0.785, cooldown 1.2,
// leech_percent 5), enemy contact-attack cadence (grunt: 5 dmg, range 28, 1.0s cooldown,
// first hit immediate), kill credit → XP, and player death.

use bevy::math::Vec2;
use rust_game::enemy::archetypes::archetypes;
use rust_game::player::components::Experience;
use rust_game::sim::Sim;

#[test]
fn death_strike_hits_only_in_cone_and_leeches() {
    let mut sim = Sim::new_arena(42);
    sim.set_player_health(50.0);

    // In cone: dead ahead at 32 units. Out of arc: 90° off at 32. Out of range: 96 ahead.
    let in_cone = sim.spawn_grunt((1, 0));
    let out_of_arc = sim.spawn_grunt((0, 1));
    let out_of_range = sim.spawn_grunt((3, 0));

    sim.set_player_facing(Vec2::X);
    sim.trigger_ability("death_strike");
    sim.step(1);

    // The in-cone grunt (10 hp) dies to the 10-damage hit the same frame.
    assert_eq!(sim.enemy_health(in_cone), None, "in-cone grunt died and despawned");
    assert_eq!(sim.enemy_health(out_of_arc), Some(10.0), "90°-off grunt untouched");
    assert_eq!(sim.enemy_health(out_of_range), Some(10.0), "out-of-range grunt untouched");

    // Leech: 10 damage * 5% = 0.5 healed.
    assert!(
        (sim.player_health() - 50.5).abs() < 1e-3,
        "leech healed 0.5, got {}",
        sim.player_health()
    );

    // Kill credit: grunt awards 3 XP to the player.
    let player = sim.player();
    let xp = sim.world().get::<Experience>(player).unwrap();
    assert_eq!(xp.current, 3, "grunt kill granted 3 XP");
    assert_eq!(xp.level, 1);
}

#[test]
fn death_strike_cooldown_gates_repeat_casts() {
    let mut sim = Sim::new_arena(42);
    // A brute (30 hp) diagonal at ~45 units: inside the 60 cone range, outside its own
    // 32-unit contact range for the duration of the test.
    let brute = sim.spawn_enemy(&archetypes()[2], (1, 1));
    sim.set_player_facing(Vec2::new(1.0, 1.0));

    sim.trigger_ability("death_strike");
    sim.step(1);
    assert_eq!(sim.enemy_health(brute), Some(20.0), "first cast lands (30-10)");

    // Immediately re-trigger: cooldown (1.2s) blocks it.
    sim.trigger_ability("death_strike");
    sim.step(1);
    assert_eq!(sim.enemy_health(brute), Some(20.0), "second cast suppressed by cooldown");

    // After 1.2s the cooldown is ready again.
    sim.step(72);
    sim.trigger_ability("death_strike");
    sim.step(1);
    assert_eq!(sim.enemy_health(brute), Some(10.0), "cast lands again after cooldown");
}

#[test]
fn unregistered_behavior_skips_gracefully() {
    let mut sim = Sim::new_arena(42);
    let grunt = sim.spawn_grunt((1, 0));
    sim.set_player_facing(Vec2::X);

    // dnd is loaded but its "dropped_zone" behavior is unregistered until Phase 6 —
    // triggering it must warn + skip, not panic or deal damage.
    sim.trigger_ability("dnd");
    sim.step(1);
    assert_eq!(sim.enemy_health(grunt), Some(10.0), "dnd is inert in Phase 2");
}

#[test]
fn grunt_contact_attack_cadence() {
    let mut sim = Sim::new_arena(42);
    // On top of the player: within the 28-unit contact range. First hit is immediate
    // (AttackCooldown starts ready), then once per 1.0s.
    sim.spawn_grunt((0, 0));

    sim.step(1);
    assert_eq!(sim.player_health(), 95.0, "first contact hit lands immediately (100-5)");

    sim.step(30); // 0.5s — still inside the 1s cooldown
    assert_eq!(sim.player_health(), 95.0, "no second hit inside the cooldown");

    sim.step(40); // total ~1.18s since first hit
    assert_eq!(sim.player_health(), 90.0, "second hit after the 1s cooldown");
}

#[test]
fn player_despawns_on_death() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    sim.deal_damage(player, 150.0);
    sim.step(2);
    assert!(sim.try_player().is_none(), "player despawned at 0 hp");
}
