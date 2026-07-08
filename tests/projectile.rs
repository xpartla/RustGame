// Golden scenarios — travelling projectiles + status-on-impact (Phase 3D).
//
// Locks in: a projectile deals its effects on IMPACT (after travel time), not at cast; Fireblast
// applies blaze and (being Fire) clears frostbite on the enemy it hits; Frostbolt applies
// frostbite and (being Frost) clears blaze; a melee cone (Scratch) applies bleed to every enemy
// it hits. Tuning is read from the *.ability.ron files (fireblast: 8 dmg / speed 320 / radius 8).

use bevy::math::Vec2;
use rust_game::sim::Sim;

#[test]
fn fireblast_projectile_travels_then_hits_and_applies_blaze() {
    let mut sim = Sim::new_arena(42);
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("fireblast");
    sim.step(1); // spawn_unlocked_ability creates the instance
    sim.set_player_facing(Vec2::X);

    let enemy = sim.spawn_grunt((8, 0)); // 256 units dead ahead
    sim.set_health(enemy, 100.0);
    sim.trigger_ability("fireblast");
    sim.step(1); // projectile spawned (deferred); has not travelled yet

    // Shortly after: the projectile is still in flight — the far enemy is untouched.
    sim.step(5);
    assert_eq!(sim.enemy_health(enemy), Some(100.0), "no instant hit — projectile in flight");
    assert!(!sim.has_status(enemy, "blaze"), "blaze not applied before impact");

    // After travel time: impact deals 8 Fire damage and applies blaze.
    sim.step(60);
    assert_eq!(sim.enemy_health(enemy), Some(92.0), "8 fire damage on impact");
    assert!(sim.has_status(enemy, "blaze"), "blaze applied on impact");
}

#[test]
fn fireblast_impact_clears_frostbite() {
    let mut sim = Sim::new_arena(42);
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("fireblast");
    sim.step(1);
    sim.set_player_facing(Vec2::X);

    let player = sim.player();
    let enemy = sim.spawn_grunt((4, 0)); // 128 units ahead
    sim.set_health(enemy, 100.0);
    sim.apply_status(enemy, player, "frostbite", 1);
    sim.step(2);
    assert!(sim.has_status(enemy, "frostbite"), "frostbite applied");

    sim.trigger_ability("fireblast");
    sim.step(40); // projectile reaches the enemy
    assert!(!sim.has_status(enemy, "frostbite"), "fireblast (Fire) cleared frostbite on impact");
    assert!(sim.has_status(enemy, "blaze"), "and applied blaze");
}

#[test]
fn frostbolt_impact_clears_blaze() {
    let mut sim = Sim::new_arena(42);
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("frostbolt");
    sim.step(1);
    sim.set_player_facing(Vec2::X);

    let player = sim.player();
    let enemy = sim.spawn_grunt((4, 0));
    sim.set_health(enemy, 100.0);
    sim.apply_status(enemy, player, "blaze", 1);
    sim.step(2);
    assert!(sim.has_status(enemy, "blaze"), "blaze applied");

    sim.trigger_ability("frostbolt");
    sim.step(40);
    assert!(!sim.has_status(enemy, "blaze"), "frostbolt (Frost) cleared blaze on impact");
    assert!(sim.has_status(enemy, "frostbite"), "and applied frostbite");
}

#[test]
fn projectile_without_pierce_stops_at_the_first_enemy() {
    let mut sim = Sim::new_arena(42);
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("fireblast");
    sim.step(1);
    sim.set_player_facing(Vec2::X);

    let near = sim.spawn_grunt((3, 0)); // 96 units ahead
    let far = sim.spawn_grunt((6, 0)); // 192 units, same line
    sim.set_health(near, 100.0);
    sim.set_health(far, 100.0);

    sim.trigger_ability("fireblast");
    // Impact at ~17 frames (96 units at 320 u/s, target closing). Probe at 45: past any
    // possible impact on either enemy, before blaze's first DoT tick (impact + 60 frames).
    sim.step(45);

    assert_eq!(sim.enemy_health(near), Some(92.0), "first enemy hit");
    assert_eq!(sim.enemy_health(far), Some(100.0), "pierce 0: the shot despawned at the first hit");
}

#[test]
fn pierced_projectile_passes_through_and_hits_one_more_enemy() {
    let mut sim = Sim::new_arena(42);
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("fireblast");
    sim.step(1);
    sim.set_player_facing(Vec2::X);
    // Test-only tuning override: give Fireblast one pierce.
    sim.set_ability_param("fireblast", "pierce", 1.0);

    let near = sim.spawn_grunt((3, 0));
    let far = sim.spawn_grunt((6, 0));
    sim.set_health(near, 100.0);
    sim.set_health(far, 100.0);

    sim.trigger_ability("fireblast");
    // Second impact at ~33 frames (192 units); probe before the first blaze tick (~impact+60).
    sim.step(45);

    assert_eq!(sim.enemy_health(near), Some(92.0), "first enemy hit");
    assert_eq!(sim.enemy_health(far), Some(92.0), "pierce 1: the shot continued to the second enemy");
    assert!(
        sim.has_status(near, "blaze") && sim.has_status(far, "blaze"),
        "the carried effects apply at every impact"
    );
}

#[test]
fn scratch_cone_deals_physical_damage_to_all_hits_without_bleed_by_default() {
    // Phase 9.4 correction: bleed is the Enhanced-attack state's own consequence (Mechanics:
    // "Enhanced — Scratch applies a bleed"), not Scratch's unconditional baseline — see
    // tests/druid.rs for the Enhanced-consumes-a-charge-and-bleeds scenario. The DK player used
    // here carries no `Charges` component at all (HealthBased resource model), so this also proves
    // a non-Charges hero casting Scratch never panics/bleeds.
    let mut sim = Sim::new_arena(42);
    sim.disable_companion(); // Phase 9.2: isolate Scratch's own damage from the DK's pet
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("scratch");
    sim.step(1);
    sim.set_player_facing(Vec2::X);

    let near = sim.spawn_grunt((1, 0)); // 32 units ahead, in the 70-range cone
    let far = sim.spawn_grunt((2, 0)); // 64 units ahead, in the cone
    sim.set_health(near, 100.0);
    sim.set_health(far, 100.0);

    sim.trigger_ability("scratch");
    sim.step(1);

    assert_eq!(sim.enemy_health(near), Some(93.0), "7 physical damage to near");
    assert_eq!(sim.enemy_health(far), Some(93.0), "7 physical damage to far");
    assert!(!sim.has_status(near, "bleed") && !sim.has_status(far, "bleed"), "no bleed without Enhanced");
}
