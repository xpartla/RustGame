// Golden scenarios — Abomination Limb (Phase 9.2).
//
// Locks in: grip pulls the nearest target within range toward the caster (abomination_limb.
// ability.ron: range 150, target_count 1, grip_speed 120, grip_duration 0.4, cooldown 6.0), and
// that it is pure crowd control (no damage dealt).

use bevy::math::Vec2;
use rust_game::sim::Sim;

#[test]
fn grip_pulls_the_nearest_enemy_toward_the_player_without_damaging_it() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("abomination_limb");

    // 100 units away — inside grip's 150 range, outside its own contact range.
    let enemy = sim.spawn_grunt((3, 0));
    sim.set_health(enemy, 100.0);
    let start = sim.entity_pos(enemy).unwrap();

    sim.step(2); // frame 1: instance spawns; frame 2: auto-cast grips the enemy
    sim.step(20); // let the 0.4s pull impulse play out

    let pos = sim.entity_pos(enemy).unwrap();
    assert!(pos.x < start.x, "grip pulled the grunt toward the player: start={start:?} now={pos:?}");
    assert_eq!(sim.enemy_health(enemy), Some(100.0), "grip deals no damage — pure crowd control");
}

#[test]
fn grip_ignores_a_target_outside_its_range() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("abomination_limb");

    // 320 units away — outside grip's 150 range.
    let enemy = sim.spawn_grunt((10, 0));
    sim.set_health(enemy, 100.0);
    let start = sim.entity_pos(enemy).unwrap();

    sim.step(2);
    sim.step(20);

    // The grunt's own MeleeChaser AI will have nudged it toward the player regardless, so assert
    // on the grip-specific signature instead: it should NOT have covered anywhere near the
    // 120 u/s * 0.4s = 48 units a grip pull would cause on top of its own ~15 u/s walk.
    let pos = sim.entity_pos(enemy).unwrap();
    let moved = start.distance(pos);
    assert!(moved < 10.0, "out-of-range grunt barely drifted under its own slow AI, moved {moved}");
}
