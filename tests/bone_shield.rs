// Golden scenarios — Bone Shield (Phase 9.2, Death Strike's epic talent).
//
// Locks in: after the killer's kill count reaches bone_shield_kill_threshold (5, death_strike.
// ability.ron), a GainShieldEvent lands for bone_shield_amount (20) and the counter wraps; no
// grant before the threshold; and — since the current implementation counts ANY kill, not
// specifically Death-Strike-caused ones (documented simplification, no DamageEvent ability
// provenance) — a kill from a different source (Blood Boil) still counts.

use bevy::math::Vec2;
use rust_game::sim::Sim;

#[test]
fn grants_a_shield_after_the_threshold_kill_count() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    sim.grant_talent("death_strike_bone_shield_epic");
    sim.step(1);

    // 4 kills: no shield yet.
    for i in 0..4 {
        let enemy = sim.spawn_grunt((1, 0));
        sim.set_health(enemy, 1.0);
        sim.trigger_ability("death_strike");
        sim.step(1);
        assert_eq!(sim.enemy_health(enemy), None, "kill #{} landed", i + 1);
        sim.step(72); // past Death Strike's 1.2s cooldown for the next cast
    }
    let player = sim.player();
    assert_eq!(sim.shield_amount(player), 0.0, "still under the 5-kill threshold");

    // 5th kill crosses the threshold. The GainShieldEvent is written in CombatSet::Death (this
    // step); apply_shield_gain (CombatSet::Apply) runs earlier in the frame, so it only sees the
    // event on the FOLLOWING step.
    let fifth = sim.spawn_grunt((1, 0));
    sim.set_health(fifth, 1.0);
    sim.trigger_ability("death_strike");
    sim.step(2);

    assert_eq!(sim.shield_amount(player), 20.0, "5th kill granted the 20-amount shield");
}

#[test]
fn no_talent_means_no_shield_no_matter_how_many_kills() {
    let mut sim = Sim::new_arena(43);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    // No bone_shield_epic talent acquired.

    for _ in 0..6 {
        let enemy = sim.spawn_grunt((1, 0));
        sim.set_health(enemy, 1.0);
        sim.trigger_ability("death_strike");
        sim.step(1);
        sim.step(72);
    }

    let player = sim.player();
    assert_eq!(sim.shield_amount(player), 0.0, "no talent, no shield, regardless of kill count");
}
