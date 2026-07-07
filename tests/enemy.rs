// Golden scenarios — enemies (Phase 5).
//
// Locks in: data-driven EnemyDef spawns (declared stats + contact ability + faction), faction-aware
// targeting (enemy contact hits the player, not other enemies; player casts don't self-hit), the
// ranged caster (approach → stop → shoot the player), enemy projectiles hitting only the player,
// and the data-only enemy-scaling model (health + damage grow with depth). Contact-attack cadence
// itself is locked by tests/combat.rs::grunt_contact_attack_cadence.

use bevy::math::Vec2;
use rust_game::core::components::{Faction, Hurtbox, MoveSpeed};
use rust_game::enemy::components::XpReward;
use rust_game::sim::Sim;

#[test]
fn enemy_def_spawns_with_declared_stats() {
    let mut sim = Sim::new_arena(1);
    let grunt = sim.spawn_enemy("grunt", (5, 0));

    assert_eq!(sim.enemy_health(grunt), Some(10.0), "grunt.enemy.ron max_health");
    assert_eq!(sim.faction(grunt), Some(Faction::Hostile), "enemies are Hostile");
    assert_eq!(
        sim.enemy_ability_ids(grunt),
        vec!["grunt_contact".to_string()],
        "grunt carries its contact ability as an AbilityInstance"
    );

    let move_speed = sim.world().get::<MoveSpeed>(grunt).unwrap().0;
    let hurtbox = sim.world().get::<Hurtbox>(grunt).unwrap().radius;
    let xp = sim.world().get::<XpReward>(grunt).unwrap().0;
    assert_eq!(move_speed, 15.0, "move_speed");
    assert_eq!(hurtbox, 12.0, "size_radius → Hurtbox");
    assert_eq!(xp, 3, "xp_value");
}

#[test]
fn enemy_contact_hits_player_not_other_enemies() {
    let mut sim = Sim::new_arena(2);
    let attacker = sim.spawn_grunt((0, 0)); // on the player, inside contact range
    let bystander = sim.spawn_grunt((5, 0)); // 160 units away, out of contact range
    sim.step(1);

    assert_eq!(sim.player_health(), 195.0, "grunt contact hit the Friendly player (200-5)");
    assert_eq!(sim.enemy_health(bystander), Some(10.0), "contact did not hit another Hostile");
    assert_eq!(sim.enemy_health(attacker), Some(10.0), "and did not hit itself");
}

#[test]
fn player_abilities_still_only_hit_enemies() {
    let mut sim = Sim::new_arena(5);
    let enemy = sim.spawn_grunt((1, 0)); // 32 units ahead — inside Death Strike's 60 cone, outside contact
    sim.set_player_facing(Vec2::X);
    let hp_before = sim.player_health();

    sim.trigger_ability("death_strike");
    sim.step(1);

    assert_eq!(sim.enemy_health(enemy), None, "Death Strike killed the Hostile (10 dmg vs 10 hp)");
    assert_eq!(sim.player_health(), hp_before, "player unharmed by its own cast (no self-hit)");
}

#[test]
fn ranged_caster_stops_at_range_and_shoots() {
    let mut sim = Sim::new_arena(3);
    // 128 units away — already inside the Spitter's 140 stand-off, so it holds position and fires.
    let spitter = sim.spawn_enemy("spitter", (4, 0));
    let hp_before = sim.player_health();
    sim.step_seconds(2.5);

    assert!(
        sim.player_health() < hp_before,
        "spitter bolt reached and damaged the player, got {}",
        sim.player_health()
    );
    let ppos = sim.player_pos();
    let spos = sim.entity_pos(spitter).expect("spitter still alive");
    let dist = spos.distance(ppos);
    assert!(dist > 100.0, "spitter held its stand-off distance instead of closing to melee, got {dist}");
}

#[test]
fn enemy_projectile_hits_player_through_a_hostile() {
    let mut sim = Sim::new_arena(4);
    // Spitter at 128 units; a grunt directly in the bolt's path at 64 units. The bolt (260 u/s)
    // reaches the player in ~0.5s, well before the slow grunt (15 u/s) closes to contact.
    let _spitter = sim.spawn_enemy("spitter", (4, 0));
    let blocker = sim.spawn_grunt((2, 0));
    let hp_before = sim.player_health();
    sim.step_seconds(0.7);

    assert_eq!(
        sim.enemy_health(blocker),
        Some(10.0),
        "enemy bolt ignored the Hostile blocker in its path (faction filter)"
    );
    assert!(
        sim.player_health() < hp_before,
        "enemy bolt passed through the enemy and hit the Friendly player"
    );
}

#[test]
fn enemy_scaling_scales_health_and_damage() {
    let mut sim = Sim::new_arena(6);
    // Data-only scaling: depth 0 == base, depth 4 grows by the grunt's per-depth curve.
    let base = sim.spawn_enemy_at_depth("grunt", (5, 0), 0);
    let deep = sim.spawn_enemy_at_depth("grunt", (6, 0), 4);

    assert_eq!(sim.enemy_health(base), Some(10.0), "depth 0 == base 10 hp");
    let deep_hp = sim.enemy_health(deep).unwrap();
    assert!((deep_hp - 16.0).abs() < 1e-3, "depth-4 grunt: 10 * (1 + 0.15*4) = 16 hp, got {deep_hp}");

    // Damage scaling reaches the player via DamageDealtModifier: 5 * (1 + 0.12*4) = 7.4.
    let _hitter = sim.spawn_enemy_at_depth("grunt", (0, 0), 4);
    sim.step(1);
    let hp = sim.player_health();
    assert!((hp - 192.6).abs() < 1e-3, "depth-4 contact hit for 7.4 (200-7.4), got {hp}");
}

#[test]
fn suppressed_caster_cannot_cast() {
    let mut sim = Sim::new_arena(7);
    let grunt = sim.spawn_grunt((0, 0)); // on the player, inside contact range

    // Stun the grunt: suppress_abilities (+ immobilize). The marker lands after one frame
    // (resolve_actor_status lag), so the grunt still lands its pre-suppression hit this step.
    sim.apply_status(grunt, grunt, "stun", 1);
    sim.step(1);

    // From here the grunt is suppressed. Over the next second a normal grunt would land another
    // contact hit; a suppressed one lands none.
    let hp = sim.player_health();
    sim.step(60);
    assert_eq!(sim.player_health(), hp, "suppressed (stunned) grunt did not contact-attack for 1s");

    // Stun (1.5s) expires; the grunt resumes attacking.
    sim.step(90);
    assert!(
        sim.player_health() < hp,
        "grunt resumed casting once suppression lifted, hp {}",
        sim.player_health()
    );
}
