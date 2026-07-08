// Golden scenarios — the Paladin (Phase 9.3, the arc's first brand-new hero).
//
// Locks in the new engine primitives and their first consumers:
//   - hammer_cleave: one full-damage primary hit + a 50%-damage cleave cone behind it.
//   - channel_while_moving: Flash of Light heals only once its cast_time channel completes, and
//     the overheal→shield talent.
//   - orbiting + the holy-mark READ path: Spinning Hammer deals double damage to marked targets.
//   - Smite is the holy-mark GRANT path, plus its zone-spawn talent.
//   - Consecrated Ground's slow talent (the new ZoneEffects.slow_status wiring).
//   - the hero-aware band-pool fix (progression/systems/level_up.rs): selecting Paladin via the
//     real character-select/run-start path grants ITS band kit, not the Death Knight's.
//
// Tuning is read from the RON assets: hammer_of_justice damage 22 / cleave_fraction 0.5;
// flash_of_light cast_time 1.2s / heal_percent 20%; spinning_hammer damage 4.0; smite applies
// holy_mark. Most of these tests use `grant_ability`/`grant_talent` directly on the default
// Death-Knight-identified sim player (the established pattern for exercising one ability/talent in
// isolation — Health.max stays the DK's 200, which is irrelevant to what each test is checking).

use bevy::math::Vec2;
use rust_game::core::components::WorldPosition;
use rust_game::sim::Sim;

#[test]
fn hammer_of_justice_hits_the_primary_in_full_and_the_cleave_target_for_half() {
    let mut sim = Sim::new_arena(60);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let primary = sim.spawn_grunt((1, 0)); // 32 units ahead — inside range 55 / half_angle
    let cleave = sim.spawn_enemy("brute", (2, 0)); // 64 units — out of primary range, but in the
                                                    // cleave cone behind the primary (32 units past it)
    for e in [primary, cleave] {
        sim.set_health(e, 1000.0);
    }

    sim.grant_ability("hammer_of_justice");
    sim.step(1); // spawn_unlocked_ability
    sim.trigger_ability("hammer_of_justice");
    sim.step(1);

    assert_eq!(sim.enemy_health(primary), Some(978.0), "primary takes the full 22 damage");
    assert_eq!(sim.enemy_health(cleave), Some(989.0), "cleave target takes 50% (11 damage)");
}

#[test]
fn hammer_of_justice_whiffs_with_nothing_in_the_primary_arc() {
    let mut sim = Sim::new_arena(66);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let out_of_range = sim.spawn_grunt((3, 0)); // 96 units — beyond range 55
    sim.set_health(out_of_range, 100.0);

    sim.grant_ability("hammer_of_justice");
    sim.step(1);
    sim.trigger_ability("hammer_of_justice");
    sim.step(1);

    assert_eq!(sim.enemy_health(out_of_range), Some(100.0), "no primary in arc — a clean whiff");
}

#[test]
fn flash_of_light_heals_only_once_the_channel_completes() {
    let mut sim = Sim::new_arena(61);
    sim.disable_companion();
    sim.grant_ability("flash_of_light");
    sim.step(1);
    sim.set_player_health(100.0); // 200 max (DK base_stats) — well clear of overheal

    sim.trigger_ability("flash_of_light");
    sim.step(1); // execute_ready_abilities inserts Channeling this frame; no heal yet
    assert_eq!(sim.player_health(), 100.0, "no heal the instant the channel starts");

    sim.step_seconds(1.0); // still mid-channel (cast_time 1.2s)
    assert_eq!(sim.player_health(), 100.0, "still channeling — no heal yet");

    sim.step_seconds(0.3); // past cast_time
    assert!(
        (sim.player_health() - 140.0).abs() < 1e-3,
        "heal_percent 20% of 200 max = 40 healed on completion, got {}",
        sim.player_health()
    );
}

#[test]
fn flash_of_light_overheal_becomes_a_shield_with_the_talent() {
    let mut sim = Sim::new_arena(62);
    sim.disable_companion();
    let player = sim.player();
    sim.grant_ability("flash_of_light");
    sim.step(1);
    sim.grant_talent("flash_of_light_overheal_shield_common");
    sim.step(1);

    sim.set_player_health(190.0); // 200 max; heal 40 -> would overheal by 30
    sim.trigger_ability("flash_of_light");
    sim.step(1);
    sim.step_seconds(1.3); // past cast_time (1.2s)

    assert_eq!(sim.player_health(), 200.0, "healed up to max, clamped");
    assert!(
        (sim.shield_amount(player) - 30.0).abs() < 1e-3,
        "the 30 overheal became a shield, got {}",
        sim.shield_amount(player)
    );
}

#[test]
fn spinning_hammer_deals_double_damage_to_holy_marked_targets() {
    let mut sim = Sim::new_arena(63);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    let player = sim.player();

    let marked = sim.spawn_grunt((1, 0));
    let unmarked = sim.spawn_grunt((1, 1));
    for e in [marked, unmarked] {
        sim.set_health(e, 10_000.0);
        sim.apply_status(e, player, "root", 1); // hold still — isolates the hammer's own sweep
    }
    // Both entities sit at the EXACT same point on the hammer's orbit path, so they are swept the
    // identical number of times — isolating the holy-mark multiplier from sweep-timing geometry.
    {
        let world = sim.world_mut();
        world.get_mut::<WorldPosition>(marked).unwrap().0 = Vec2::new(45.0, 0.0);
        world.get_mut::<WorldPosition>(unmarked).unwrap().0 = Vec2::new(45.0, 0.0);
    }
    sim.apply_status(marked, player, "holy_mark", 1);
    sim.step(2); // let root + holy_mark fully resolve BEFORE the hammer's first cast can see them

    // Granted only now, so its first auto-cast (next frame) already sees the settled mark/root.
    sim.grant_ability("spinning_hammer");
    // orbit_radius 45 / angular_speed 3.0 rad/s -> a full rotation is ~2.09s; step well past one
    // full sweep so the hammer is GUARANTEED to pass angle 0 (where both targets sit) at least once,
    // regardless of which point in its cycle the very first cast happens to land on.
    sim.step_seconds(3.0);

    let marked_dmg = 10_000.0 - sim.enemy_health(marked).unwrap();
    let unmarked_dmg = 10_000.0 - sim.enemy_health(unmarked).unwrap();
    assert!(unmarked_dmg > 0.0, "the hammer swept past and hit the unmarked control, got {unmarked_dmg}");
    assert!(
        (marked_dmg - 2.0 * unmarked_dmg).abs() < 1e-3,
        "the marked target took exactly double: {marked_dmg} vs {unmarked_dmg}"
    );
}

#[test]
fn smite_applies_holy_mark_to_its_target() {
    let mut sim = Sim::new_arena(64);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    let target = sim.spawn_grunt((1, 0));
    sim.set_health(target, 1000.0);

    sim.grant_ability("smite");
    sim.step(2); // spawn instance, then auto-cast fires (target already in place)

    assert!(sim.has_status(target, "holy_mark"), "Smite applies holy_mark — the holy-mark GRANT path");
    assert!(sim.enemy_health(target).unwrap() < 1000.0, "Smite also dealt its damage");
}

#[test]
fn smite_spawns_consecrated_ground_under_the_target_with_the_talent() {
    let mut sim = Sim::new_arena(65);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    let target = sim.spawn_grunt((1, 0)); // world (32, 0)
    sim.set_health(target, 1000.0);

    sim.grant_ability("smite");
    sim.step(1);
    sim.grant_talent("smite_spawns_consecrated_rare");
    sim.step(2); // auto-cast fires now that both the talent and a target are in place

    assert!(sim.zone_types().contains(&"consecrated_ground".to_string()), "the talent spawned a zone");
    let center = sim.zone_center("consecrated_ground").expect("consecrated_ground zone exists");
    assert!(
        (center - Vec2::new(32.0, 0.0)).length() < 1.0,
        "the zone dropped under the smitten target, not the caster; got {center:?}"
    );
}

#[test]
fn consecrated_ground_slow_talent_slows_occupants() {
    let mut sim = Sim::new_arena(67);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("consecrated_ground");
    sim.step(1);
    sim.grant_talent("consecrated_ground_slow_common");
    sim.step(2); // auto-cast fires -> zone spawns at the caster (radius 60)

    assert!(sim.zone_types().contains(&"consecrated_ground".to_string()));

    let inside = sim.spawn_grunt((1, 0)); // 32 units — inside the 60-radius zone
    sim.set_health(inside, 10_000.0);

    sim.step_seconds(1.1); // past the first 1 Hz zone tick

    assert!(sim.has_status(inside, "consecrated_slow"), "the slow talent applied consecrated_slow");
}

#[test]
fn selecting_paladin_unlocks_its_own_band_kit_not_the_death_knights() {
    let mut sim = Sim::new_arena(68);
    sim.request_start_run("paladin", 68);
    sim.step(3); // apply_start_run_request -> reset_and_start_run (respawn as paladin)

    assert_eq!(sim.hero_id(), "paladin");
    let granted_at_start = sim.owned_abilities();
    assert!(granted_at_start.contains(&"hammer_of_justice".to_string()));
    assert!(granted_at_start.contains(&"flash_of_light".to_string()));

    // 10 + 15 + 20 = enough XP for three level-ups (L1 -> L4), draining Paladin's entire
    // band_2_3_pool (consecrated_ground/spinning_hammer/smite — all three, at levels 2/3/4).
    sim.grant_xp(45);
    sim.step(5);

    let owned = sim.owned_abilities();
    for id in ["consecrated_ground", "spinning_hammer", "smite"] {
        assert!(owned.contains(&id.to_string()), "Paladin's own band pool granted {id}");
    }
    for id in ["blood_boil", "heart_strike", "abomination_limb", "purgatory", "amz"] {
        assert!(
            !owned.contains(&id.to_string()),
            "the Death Knight's band pool must NOT leak onto a Paladin run ({id})"
        );
    }
}
