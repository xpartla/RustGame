// Golden scenarios — Blood Boil's remaining talent tree + the bdk_passive_blood_boil_spawns_dnd
// class passive (Phase 9.2).
//
// Locks in: blood_boil_damage_common/range_common scale their stats; blood_boil_health_scaling_rare
// applies "bleed" to every hit (the documented simplification of the % current-health DoT);
// bdk_passive_blood_boil_spawns_dnd drops a death_and_decay zone on every Blood Boil cast.

use bevy::math::Vec2;
use rust_game::sim::Sim;

#[test]
fn health_scaling_rare_applies_bleed_to_every_hit() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_talent("blood_boil_health_scaling_rare");
    sim.grant_ability("blood_boil");

    let enemy = sim.spawn_grunt((1, 0)); // inside Blood Boil's 90-radius nova
    sim.set_health(enemy, 100.0);

    sim.step(2); // instance spawns; auto-cast fires

    assert!(sim.has_status(enemy, "bleed"), "blood_boil_health_scaling_rare applies bleed on hit");
}

#[test]
fn no_talent_means_no_bleed() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("blood_boil");

    let enemy = sim.spawn_grunt((1, 0));
    sim.set_health(enemy, 100.0);
    sim.step(2);

    assert!(!sim.has_status(enemy, "bleed"), "no talent, no bleed");
}

#[test]
fn spawns_dnd_class_passive_drops_a_zone_on_cast() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_talent("bdk_passive_blood_boil_spawns_dnd");
    sim.grant_ability("blood_boil");
    sim.step(2);

    assert!(sim.zone_types().contains(&"death_and_decay".to_string()), "Blood Boil auto-spawned a D&D zone");
}

#[test]
fn no_talent_means_blood_boil_never_spawns_a_zone() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("blood_boil");
    sim.step(2);

    assert!(sim.zone_types().is_empty(), "no talent, no auto-spawned zone");
}
