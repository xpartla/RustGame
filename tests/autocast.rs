// Golden scenarios — auto-cast + the per-behavior aim gate (Phase 3E).
//
// Blood Boil (self_nova, AutoCast) fires on cooldown with no input and no aim; a needs-aim shape
// (Death Strike cone) still refuses to fire without a facing. Tuning from blood_boil.ability.ron
// (6 dmg / radius 90 / 4.0s cooldown) and death_strike.ability.ron.

use bevy::math::Vec2;
use rust_game::sim::Sim;

#[test]
fn blood_boil_auto_casts_on_cooldown_without_input_or_aim() {
    let mut sim = Sim::new_arena(42);
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("blood_boil");
    sim.step(1); // instance spawned (starts ready)

    let enemy = sim.spawn_grunt((2, 0)); // 64 units — inside the 90 radius
    sim.set_health(enemy, 100.0);
    // Note: facing is never set (stays zero). A self-nova needs no aim.

    sim.step(1); // auto_cast fires Blood Boil → 6 physical damage
    assert_eq!(sim.enemy_health(enemy), Some(94.0), "Blood Boil pulsed with no input and no aim");

    sim.step(60); // 1 s — inside the 4 s cooldown
    assert_eq!(sim.enemy_health(enemy), Some(94.0), "cooldown gates the next pulse");

    sim.step(200); // past 4 s since the first pulse
    assert_eq!(sim.enemy_health(enemy), Some(88.0), "second pulse landed after the cooldown");
}

#[test]
fn needs_aim_ability_does_not_fire_without_facing() {
    let mut sim = Sim::new_arena(42);
    let enemy = sim.spawn_grunt((1, 0)); // 32 units dead ahead, well inside Death Strike's range
    // Facing is left at zero (no mouse aim yet).
    sim.trigger_ability("death_strike");
    sim.step(1);
    assert_eq!(sim.enemy_health(enemy), Some(10.0), "the cone needs aim — no cast, no cooldown burned");

    // Once aimed, the very next trigger lands (cooldown was never consumed).
    sim.set_player_facing(Vec2::X);
    sim.trigger_ability("death_strike");
    sim.step(1);
    assert_eq!(sim.enemy_health(enemy), None, "aimed cast kills the 10-hp grunt");
}
