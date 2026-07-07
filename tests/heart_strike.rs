// Golden scenarios — Heart Strike (Phase 9.2).
//
// Locks in: nearest_melee hits up to "target_count" of the nearest enemies within "range"
// (heart_strike.ability.ron: damage 8, range 55, target_count 2, cooldown 3.0); the innate
// missing-health damage scaling (always active, not a talent — Mechanics' "increase damage as
// health lowers"); and that it auto-casts without input, like Blood Boil.
//
// Enemies are spawned BEFORE stepping past the grant: heart_strike's AbilityCooldown starts ready,
// so its auto-cast fires the moment the granted AbilityInstance exists (one frame after the grant
// event is processed, since `spawn_unlocked_ability` runs `.after(CombatSet::Death)` — see
// ability/plugin.rs) — if no target existed yet that frame, it would whiff and reset its own 3.0s
// cooldown before the scenario's own checks run.

use bevy::math::Vec2;
use rust_game::sim::Sim;

#[test]
fn hits_only_the_nearest_target_count_within_range() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("heart_strike");

    // 3 grunts within the 55-range at increasing distance, one just outside range.
    let near = sim.spawn_grunt((1, 0)); // 32 units
    let mid = sim.spawn_enemy("brute", (1, 1)); // ~45 units, diagonal
    let out_of_range = sim.spawn_grunt((2, 0)); // 64 units — outside range
    for e in [near, mid, out_of_range] {
        sim.set_health(e, 100.0);
    }

    sim.step(2); // frame 1: instance spawns; frame 2: auto-cast fires (targets already in place)

    // target_count defaults to 2 — only the 2 nearest (near, mid) are hit; the full-health caster
    // takes no missing-health bonus, so the flat 8 damage lands.
    assert_eq!(sim.enemy_health(near), Some(92.0), "nearest hit for 8 dmg");
    assert_eq!(sim.enemy_health(mid), Some(92.0), "2nd-nearest hit for 8 dmg");
    assert_eq!(sim.enemy_health(out_of_range), Some(100.0), "out-of-range grunt untouched");
}

#[test]
fn damage_scales_up_as_the_casters_health_drops() {
    let mut sim = Sim::new_arena(43);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_health(200.0); // full DK health (Phase 9.2 base_stats) — no missing-health bonus
    sim.grant_ability("heart_strike");

    let target = sim.spawn_grunt((1, 0));
    sim.set_health(target, 1000.0); // durable dummy
    let player = sim.player();
    // Stun the grunt (immobilize + suppress_abilities) so it can neither close the remaining 4
    // units to its own 28-range contact attack nor land one — isolating heart_strike's own
    // health-scaling from incidental contact damage over the ~3s window below.
    sim.apply_status(target, player, "stun", 1);

    sim.step(2); // frame 1: instance spawns; frame 2: auto-cast fires at full health: plain 8 dmg
    let after_full = sim.enemy_health(target).unwrap();
    assert!((1000.0 - after_full - 8.0).abs() < 1e-3, "full health: plain 8 dmg, got {after_full}");

    // Half health: +50% missing-health bonus -> 12 damage on the next cast (3.0s cooldown). Stun
    // (1.5s) will have worn off by then, so refresh it to keep the grunt from contact-attacking.
    sim.set_player_health(100.0);
    sim.apply_status(target, player, "stun", 1);
    sim.step_seconds(1.0);
    sim.apply_status(target, player, "stun", 1);
    sim.step_seconds(1.0);
    sim.apply_status(target, player, "stun", 1);
    sim.step_seconds(1.1);
    let after_half = sim.enemy_health(target).unwrap();
    let second_hit = after_full - after_half;
    assert!((second_hit - 12.0).abs() < 1e-3, "half health: +50% -> 12 dmg, got {second_hit}");
}
