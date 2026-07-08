// Golden scenarios — the Mage completion (Phase 9.5, the arc's fourth and final class kit).
//
// Locks in the new engine primitives and their first consumers:
//   - Frostbolt's innate frost-charge generation (ProjectilePayload.grants_frost_charge_on_
//     frostbitten), checked BEFORE the hit's own ApplyStatus(frostbite) lands so it only fires
//     against a target frostbitten by a PRIOR cast.
//   - Fireblast's "explodes on impact" unique talent (ProjectilePayload.explode_on_impact),
//     the projectile-impact talent special-case gap the Phase 9.4 as-built notes flagged.
//   - `targeted_burst` (Flamestrike): an aimed AoE offset from the caster, with a base-kit
//     "increased damage per blazed enemy present" top-up.
//   - Flamewrath reusing `self_nova` verbatim, with its own damage/blaze-consumption entirely a
//     targeted execute.rs special-case (empty `effects` list).
//   - Frost Impale's icicle: `channel_while_moving` extended to fire a piercing projectile at
//     completion, consuming every held frost Charge and scaling damage per charge spent.
//   - The two Frostbite-passive kill-reactive class talents (mirrors bone_shield_on_kill's shape).
//
// Most tests grant abilities/talents directly onto the default Death-Knight-identified sim player
// (the established Paladin/Druid-file pattern) and attach `Charges` via `Sim::set_charges` where
// frost-charge state matters — Health.max stays the DK's 200, irrelevant to what's being checked.
// The band-pool scenario needs a real Mage identity (`Sim::request_start_run`).

use bevy::math::Vec2;
use rust_game::sim::Sim;

#[test]
fn frostbolt_generates_a_frost_charge_only_when_the_target_is_already_frostbitten() {
    let mut sim = Sim::new_arena(90);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let player = sim.player();
    sim.set_charges(player, 0, 10);

    let target = sim.spawn_grunt((5, 0)); // 160 units ahead, inside Frostbolt's 400 range
    sim.set_health(target, 1000.0);

    sim.grant_ability("frostbolt");
    sim.step(1);
    sim.trigger_ability("frostbolt");
    sim.step_seconds(0.6); // travel (160 / 300 speed) + impact

    assert!(sim.has_status(target, "frostbite"), "first hit applies frostbite");
    assert_eq!(sim.class_resource(player), Some((0.0, 10.0)), "no charge yet — the target wasn't already frostbitten");

    sim.step_seconds(1.1); // Frostbolt's own 1.0s cooldown
    sim.trigger_ability("frostbolt");
    sim.step_seconds(0.6);

    assert_eq!(
        sim.class_resource(player),
        Some((1.0, 10.0)),
        "hitting an ALREADY-frostbitten target granted a charge"
    );
}

#[test]
fn fireblast_explode_on_impact_talent_damages_a_nearby_enemy_too() {
    let mut sim = Sim::new_arena(91);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);

    let primary = sim.spawn_grunt((5, 0)); // world (160, 0) — the direct hit
    let nearby = sim.spawn_enemy("brute", (5, 1)); // world (160, 32) — inside explode_radius 50
    for e in [primary, nearby] {
        sim.set_health(e, 1000.0);
    }

    sim.grant_ability("fireblast");
    sim.step(1);
    sim.grant_talent("fireblast_explode_on_impact_common");
    sim.step(1);
    sim.trigger_ability("fireblast");
    sim.step_seconds(0.6); // travel (160 / 320 speed) + impact

    assert_eq!(sim.enemy_health(primary), Some(992.0), "primary takes only the base 8 damage, not the explosion too");
    assert_eq!(sim.enemy_health(nearby), Some(994.0), "nearby enemy takes the 6 explosion damage");
}

#[test]
fn flamewrath_explodes_around_the_nearest_ablaze_target_and_consumes_its_blaze() {
    let mut sim = Sim::new_arena(92);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    let player = sim.player();

    let ablaze = sim.spawn_grunt((3, 0)); // world (96, 0) — inside Flamewrath's 220 search radius
    let nearby = sim.spawn_enemy("brute", (3, 1)); // world (96, 32) — inside explosion_radius 70
    for e in [ablaze, nearby] {
        sim.set_health(e, 1000.0);
    }
    sim.apply_status(ablaze, player, "blaze", 1);
    sim.step(1); // let blaze settle before Flamewrath's first auto-cast can see it

    sim.grant_ability("flamewrath");
    sim.step_seconds(0.2); // AutoCast fires on its own cooldown

    assert_eq!(sim.enemy_health(ablaze), Some(993.0), "the ablaze target takes the 7 explosion damage");
    assert_eq!(sim.enemy_health(nearby), Some(993.0), "the nearby enemy is caught in the same explosion");
    assert!(!sim.has_status(ablaze, "blaze"), "Flamewrath consumed the blaze stack");
}

#[test]
fn flamewrath_no_consume_talent_halves_damage_but_keeps_the_blaze_stack() {
    let mut sim = Sim::new_arena(93);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    let player = sim.player();

    let ablaze = sim.spawn_grunt((3, 0));
    sim.set_health(ablaze, 1000.0);
    sim.apply_status(ablaze, player, "blaze", 1);
    sim.step(1);

    sim.grant_ability("flamewrath");
    sim.step(1);
    sim.grant_talent("flamewrath_no_consume_common");
    sim.step_seconds(0.2);

    assert_eq!(sim.enemy_health(ablaze), Some(996.5), "halved damage: 3.5 instead of 7");
    assert!(sim.has_status(ablaze, "blaze"), "the no-consume talent kept the blaze stack alive");
}

#[test]
fn flamestrike_deals_bonus_damage_per_blazed_enemy_present_to_every_hit() {
    let mut sim = Sim::new_arena(94);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let player = sim.player();

    // cast_range 180 along +X -> blast center (180, 0); zone_radius 60.
    let blazed = sim.spawn_grunt((6, 0)); // world (192, 0) — 12 units from the center
    let unblazed = sim.spawn_enemy("brute", (5, 1)); // world (160, 32) — ~37.7 units from the center
    for e in [blazed, unblazed] {
        sim.set_health(e, 1000.0);
    }
    sim.apply_status(blazed, player, "blaze", 1);
    sim.step(1);

    sim.grant_ability("flamestrike");
    sim.step(1);
    sim.trigger_ability("flamestrike");
    sim.step(1);

    // Base 10 damage to both hits, plus a +15%-of-10-per-blazed-enemy-present top-up (one blazed
    // enemy present -> +1.5) applied to EVERY hit, not just the blazed one.
    assert_eq!(sim.enemy_health(blazed), Some(988.5), "10 base + 1.5 bonus");
    assert_eq!(sim.enemy_health(unblazed), Some(988.5), "the bonus is global to the cast, not per-target");
}

#[test]
fn frost_impale_channel_fires_an_icicle_scaling_with_consumed_charges() {
    let mut sim = Sim::new_arena(95);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let player = sim.player();
    sim.set_charges(player, 4, 10);

    let target = sim.spawn_grunt((10, 0)); // world (320, 0) — within the icicle's ~500 range
    sim.set_health(target, 1000.0);

    sim.grant_ability("frost_impale");
    sim.step(1);
    sim.trigger_ability("frost_impale");
    sim.step(1); // Channeling inserted this frame; no hit yet
    assert_eq!(sim.enemy_health(target), Some(1000.0), "no damage the instant the channel starts");

    sim.step_seconds(1.6); // past cast_time (1.5s)
    assert_eq!(sim.class_resource(player), Some((0.0, 10.0)), "all 4 charges consumed on completion");

    sim.step_seconds(1.3); // icicle travels 320 units at speed 260 (~1.23s)

    // base 20 damage * (1 + 4 charges * 15%) = 20 * 1.6 = 32.
    assert_eq!(sim.enemy_health(target), Some(968.0), "damage scaled by the 4 consumed frost charges");
}

#[test]
fn frost_charge_on_frostbitten_kill_talent_grants_a_charge() {
    let mut sim = Sim::new_arena(96);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let player = sim.player();
    sim.set_charges(player, 0, 10);
    sim.grant_talent("mage_passive_frost_charge_on_frostbitten_kill_rare");
    sim.step(1);

    let enemy = sim.spawn_grunt((1, 0)); // inside Death Strike's melee cone
    sim.set_health(enemy, 5.0); // dies to Death Strike's 10 dmg
    sim.apply_status(enemy, player, "frostbite", 1);
    sim.step(1); // let the status settle before the kill

    sim.trigger_ability("death_strike");
    sim.step(1);

    assert_eq!(sim.class_resource(player), Some((1.0, 10.0)), "killing a frostbitten enemy granted a charge");
}

#[test]
fn no_frost_charge_talent_means_no_charge_from_a_frostbitten_kill() {
    let mut sim = Sim::new_arena(97);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let player = sim.player();
    sim.set_charges(player, 0, 10);

    let enemy = sim.spawn_grunt((1, 0));
    sim.set_health(enemy, 5.0);
    sim.apply_status(enemy, player, "frostbite", 1);
    sim.step(1);

    sim.trigger_ability("death_strike");
    sim.step(1);

    assert_eq!(sim.class_resource(player), Some((0.0, 10.0)), "no talent — no charge granted");
}

#[test]
fn heal_on_frostbitten_kill_talent_heals_a_percent_of_max_health() {
    let mut sim = Sim::new_arena(98);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let player = sim.player();
    sim.set_ability_param("death_strike", "leech_percent", 0.0); // isolate the frostbitten-kill heal
    sim.grant_talent("mage_passive_frostbitten_kill_heal_epic");
    sim.step(1);
    sim.set_player_health(50.0); // 200 max (DK base_stats)

    let enemy = sim.spawn_grunt((1, 0));
    sim.set_health(enemy, 5.0);
    sim.apply_status(enemy, player, "frostbite", 1);
    sim.step(1);

    sim.trigger_ability("death_strike");
    sim.step(2); // damage resolves frame 1; the Death-set HealEvent applies via frame 2's apply_heal

    assert!(
        (sim.player_health() - 60.0).abs() < 1e-3,
        "5% of 200 max = 10 healed, got {}",
        sim.player_health()
    );
}

#[test]
fn selecting_mage_unlocks_its_own_band_kit_not_the_death_knights() {
    let mut sim = Sim::new_arena(99);
    sim.request_start_run("mage", 99);
    sim.step(3); // apply_start_run_request -> reset_and_start_run (respawn as mage)

    assert_eq!(sim.hero_id(), "mage");
    let granted_at_start = sim.owned_abilities();
    for id in ["fireblast", "frostbolt", "flamestrike", "frost_impale"] {
        assert!(granted_at_start.contains(&id.to_string()), "{id} granted at level 1");
    }
    let player = sim.player();
    assert_eq!(sim.class_resource(player), Some((0.0, 10.0)), "Charges {{ max: 10 }} applied at spawn");

    // 10 + 15 + 20 = enough XP for three level-ups (L1 -> L4) — band_2_3_pool is empty (nothing to
    // grant at 2/3), and Flamewrath (band_4_6_pool's only entry) is offered starting at level 4.
    sim.grant_xp(45);
    sim.step(5);

    let owned = sim.owned_abilities();
    assert!(owned.contains(&"flamewrath".to_string()), "Mage's own band pool granted flamewrath");
    for id in ["blood_boil", "heart_strike", "abomination_limb", "purgatory", "amz", "hammer_of_justice", "scratch"] {
        assert!(!owned.contains(&id.to_string()), "no other hero's kit leaked onto a Mage run ({id})");
    }
}
