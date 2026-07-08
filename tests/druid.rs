// Golden scenarios — the Druid (Phase 9.4, the arc's hardest new hero: two forms, the
// Enhanced-attack charge state, leaps, a summon that taunts, and a pickup-driven enhancement).
//
// Locks in the new engine primitives and their first consumers:
//   - `cast_on_enter` (hero/assets.rs): entering a stance casts that stance's own Basic ability
//     (Scratch on -> Animal, Roots on -> Human) — distinct from the Mage's swap_effect status model.
//   - `leap_to_target`: cursor-nearest mode (Ferocious Bite) and highest-health mode (Primal Pounce).
//   - the Enhanced-attack state (hero::components::Charges::spend_one): Scratch/Ferocious Bite each
//     spend at most one charge per cast for their "Enhanced" bonus effect.
//   - `bloom` behavior + PickUpKind::Enhance: a pickup that grants a charge on contact.
//   - `summon`'s generalized minion body params + the Ent taunt redirect (enemy::systems::taunt).
//
// Most of these tests grant the ability directly onto the default Death-Knight-identified sim
// player (the established Paladin-file pattern) and attach `Charges` via `Sim::set_charges` where
// the Enhanced state matters — Health.max stays the DK's 200, which is irrelevant to what each test
// checks. The stance-swap scenario needs a real Druid identity (`Sim::set_hero`), and the band-pool
// scenario needs a full run-start (`Sim::request_start_run`).

use bevy::math::Vec2;
use bevy::prelude::{Entity, KeyCode, With};
use rust_game::ability::components::Minion;
use rust_game::core::components::WorldPosition;
use rust_game::enemy::components::{Taunt, Taunted};
use rust_game::sim::Sim;

#[test]
fn swapping_into_animal_form_casts_scratch_and_swapping_back_casts_roots() {
    let mut sim = Sim::new_arena(70);
    sim.disable_companion();
    let player = sim.player();
    sim.set_hero(player, "druid", "human");
    sim.step(2); // deferred level-1 grant (scratch/ferocious_bite/roots/heal) + instance spawn
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);

    let target = sim.spawn_grunt((1, 0)); // 32 units ahead, inside both Scratch's and Roots' reach
    sim.set_health(target, 1000.0);

    assert_eq!(sim.active_stance(), "human");
    sim.tap_key(KeyCode::KeyQ);
    sim.step(1);
    assert_eq!(sim.active_stance(), "animal", "Q swaps Human -> Animal");
    sim.step(1); // the cast_on_enter TriggerAbilityEvent resolves in execute_ready_abilities

    let after_scratch = sim.enemy_health(target).unwrap();
    assert!(after_scratch < 1000.0, "entering Animal form cast Scratch, damaging the target");

    sim.tap_key(KeyCode::KeyQ);
    sim.step(1);
    assert_eq!(sim.active_stance(), "human", "Q swaps Animal -> Human");
    // Roots is a projectile (unlike Scratch's instant cone) — give it time to travel the 32 units
    // to the target and collide (speed 320 => ~0.1s) before checking for its damage.
    sim.step_seconds(0.3);

    let after_roots = sim.enemy_health(target).unwrap();
    assert!(after_roots < after_scratch, "entering Human form cast Roots, dealing further damage");
}

#[test]
fn enhanced_scratch_spends_a_charge_and_bleeds_the_nearest_hits_only_once() {
    let mut sim = Sim::new_arena(71);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let player = sim.player();
    sim.set_charges(player, 1, 3);

    let near = sim.spawn_grunt((1, 0)); // 32 units, in Scratch's 70-range cone
    sim.set_health(near, 1000.0);

    sim.grant_ability("scratch");
    sim.step(1);
    sim.trigger_ability("scratch");
    sim.step(1);

    assert_eq!(sim.enemy_health(near), Some(993.0), "7 physical damage");
    assert!(sim.has_status(near, "bleed"), "Enhanced (1 charge held) applied bleed");
    assert_eq!(sim.class_resource(player), Some((0.0, 3.0)), "the charge was spent");

    // A second cast with no charges left deals only the base damage (Enhanced already spent).
    // Scratch's cooldown is 1.0s — wait for it first, THEN reset health right before the second
    // trigger (not before), so the still-active bleed's own 1 Hz DoT tick can't sneak in an extra
    // 3 damage between the reset and this cast.
    sim.step_seconds(1.1);
    sim.set_health(near, 1000.0);
    sim.trigger_ability("scratch");
    sim.step(1);
    assert_eq!(sim.enemy_health(near), Some(993.0), "still just the base 7 damage — no charge to spend");
}

#[test]
fn ferocious_bite_leaps_to_the_nearest_target_in_the_cursor_arc_and_crits_if_bleeding() {
    let mut sim = Sim::new_arena(72);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let player = sim.player();

    let bleeding = sim.spawn_grunt((2, 0)); // 64 units ahead — inside leap_range 150 / half_angle
    sim.set_health(bleeding, 1000.0);
    sim.apply_status(bleeding, player, "bleed", 1);
    sim.step(1); // let the status settle before the cast reads it

    sim.grant_ability("ferocious_bite");
    sim.step(1);
    sim.trigger_ability("ferocious_bite");
    sim.step(1);

    // base 12 damage * bleed_crit_mult 2.0 = 24 total (always-crit-if-bleeding is base kit, no talent).
    assert_eq!(sim.enemy_health(bleeding), Some(976.0), "damage is doubled against a bleeding target");
}

#[test]
fn ferocious_bite_without_bleeding_target_deals_only_base_damage() {
    let mut sim = Sim::new_arena(73);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);

    let healthy = sim.spawn_grunt((2, 0));
    sim.set_health(healthy, 1000.0);

    sim.grant_ability("ferocious_bite");
    sim.step(1);
    sim.trigger_ability("ferocious_bite");
    sim.step(1);

    assert_eq!(sim.enemy_health(healthy), Some(988.0), "base 12 damage, no bleed bonus");
}

#[test]
fn ferocious_bite_enhanced_cleaves_bleed_onto_nearby_targets() {
    let mut sim = Sim::new_arena(74);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let player = sim.player();
    sim.set_charges(player, 1, 3);

    let primary = sim.spawn_grunt((2, 0)); // 64 units ahead — the leap target
    let nearby = sim.spawn_enemy("brute", (2, 1)); // near the primary's landing spot
    for e in [primary, nearby] {
        sim.set_health(e, 1000.0);
    }

    sim.grant_ability("ferocious_bite");
    sim.step(1);
    sim.trigger_ability("ferocious_bite");
    sim.step(1);

    assert!(sim.has_status(nearby, "bleed"), "Enhanced cleaved bleed onto the nearby enemy");
    assert!(!sim.has_status(primary, "bleed"), "the primary itself isn't in its own cleave circle");
}

#[test]
fn primal_pounce_auto_leaps_to_the_highest_health_enemy_in_range() {
    let mut sim = Sim::new_arena(75);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);

    let weak_and_near = sim.spawn_grunt((1, 0)); // 32 units — nearest, but low health
    let tanky_and_far = sim.spawn_enemy("brute", (5, 0)); // 160 units — inside leap_range 180
    sim.set_health(weak_and_near, 20.0);
    sim.set_health(tanky_and_far, 900.0);

    sim.grant_ability("primal_pounce");
    sim.step(1);
    sim.step_seconds(0.2); // AutoCast fires on its own cooldown

    assert_eq!(sim.enemy_health(weak_and_near), Some(20.0), "the nearer, weaker enemy is untouched");
    assert!(
        sim.enemy_health(tanky_and_far).unwrap() < 900.0,
        "the higher-health enemy was leapt to and hit instead"
    );
    assert!(sim.has_status(tanky_and_far, "bleed"), "Primal Pounce always applies bleed, unconditionally");
}

#[test]
fn bloom_pickup_grants_an_enhanced_charge() {
    let mut sim = Sim::new_arena(76);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    let player = sim.player();
    sim.set_charges(player, 0, 3);

    sim.grant_ability("bloom");
    sim.step(1);
    sim.step_seconds(0.2); // AutoCast drops the flower at the caster's position

    // Walk over it — collect_pickups is a proximity test, and the player is already standing there.
    sim.step(1);

    assert_eq!(sim.class_resource(player), Some((1.0, 3.0)), "the flower granted 1 Enhanced charge");
}

#[test]
fn spawn_ent_taunts_a_nearby_enemy_off_the_player() {
    let mut sim = Sim::new_arena(77);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);

    sim.grant_ability("spawn_ent");
    sim.step(1);
    sim.step_seconds(0.2); // AutoCast fires, spawning the Ent at the caster's position

    let ent = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<Entity, With<Taunt>>();
        q.iter(world).next().expect("the Ent carries a Taunt component")
    };
    // Move the Ent away from the player so taunted-vs-untaunted steering is actually distinguishable.
    {
        let world = sim.world_mut();
        world.get_mut::<WorldPosition>(ent).unwrap().0 = Vec2::new(150.0, 0.0);
    }

    // Well within the Ent's 120-radius taunt range, roughly between the two.
    let enemy = sim.spawn_enemy("grunt", (3, 0)); // world (96, 0)
    sim.set_health(enemy, 1000.0);
    sim.step(2); // apply_ent_taunt runs in MovementSet::Intent, before steering

    let world = sim.world_mut();
    let taunted = world.get::<Taunted>(enemy).expect("the enemy was taunted toward the Ent");
    assert_eq!(taunted.0, ent);
}

#[test]
fn selecting_druid_unlocks_its_own_band_kit_not_the_death_knights() {
    let mut sim = Sim::new_arena(78);
    sim.request_start_run("druid", 78);
    sim.step(3); // apply_start_run_request -> reset_and_start_run (respawn as druid)

    assert_eq!(sim.hero_id(), "druid");
    let granted_at_start = sim.owned_abilities();
    for id in ["scratch", "ferocious_bite", "roots", "heal"] {
        assert!(granted_at_start.contains(&id.to_string()), "{id} granted at level 1");
    }
    let player = sim.player();
    assert_eq!(sim.class_resource(player), Some((0.0, 3.0)), "Charges {{ max: 3 }} applied at spawn");

    // 10 + 15 = enough XP for two level-ups (L1 -> L3), draining Druid's entire band_2_3_pool
    // (primal_pounce/spawn_ent).
    sim.grant_xp(25);
    sim.step(5);

    let owned = sim.owned_abilities();
    for id in ["primal_pounce", "spawn_ent"] {
        assert!(owned.contains(&id.to_string()), "Druid's own band pool granted {id}");
    }
    for id in ["blood_boil", "heart_strike", "abomination_limb", "purgatory", "amz", "hammer_of_justice"] {
        assert!(!owned.contains(&id.to_string()), "no other hero's kit leaked onto a Druid run ({id})");
    }
}

#[test]
fn minion_component_is_reused_by_the_ent_taunt_body() {
    // A quick smoke test that Spawn Ent's minion is a genuine `Minion` (reaped by the same
    // encounter-teardown/lifecycle paths as Companion), not a bespoke entity shape.
    let mut sim = Sim::new_arena(79);
    sim.disable_companion();
    sim.grant_ability("spawn_ent");
    sim.step(1);
    sim.step_seconds(0.2);

    let world = sim.world_mut();
    let mut minions = world.query_filtered::<Entity, With<Minion>>();
    assert_eq!(minions.iter(world).count(), 1, "the Ent is a Minion");
}
