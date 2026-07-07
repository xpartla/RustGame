// Golden scenarios — Companion / the `summon` ability behavior (Phase 9.2).
//
// The Blood Death Knight's level-1 Companion is a real, always-active AutoCast ability: on
// cooldown it spawns a Friendly minion that fights independently (its own AbilityInstance/
// Faction/WorldPosition — the faction-aware ability engine needed zero changes to fire a
// non-player caster's attacks). `Sim::new_arena`'s default DK already *owns* Companion, but a
// freshly settled sim hasn't necessarily let it fire yet (granting and its first eligible
// auto-cast are each their own frame, by design — see ability/plugin.rs's `.after(CombatSet::Death)`
// pin), so these tests step a few frames first. Most OTHER scenario tests instead call
// `sim.disable_companion()` to stay isolated to their own mechanic (see docs/testing.md); these
// are where the pet itself is exercised on purpose.

use bevy::math::Vec2;
use bevy::prelude::{Entity, With};
use rust_game::ability::components::{AbilityInstance, Minion};
use rust_game::core::components::Faction;
use rust_game::sim::Sim;

fn minion_entities(sim: &mut Sim) -> Vec<Entity> {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<Entity, With<Minion>>();
    q.iter(world).collect()
}

/// Steps until Companion's boot-time whiff-cast has produced a minion (bounded, so a real
/// regression fails fast instead of hanging).
fn await_first_minion(sim: &mut Sim) {
    for _ in 0..30 {
        if !minion_entities(sim).is_empty() {
            return;
        }
        sim.step(1);
    }
    panic!("Companion never cast within 30 frames of settling");
}

#[test]
fn companion_spawns_a_friendly_minion_owning_its_own_attack_instance() {
    let mut sim = Sim::new_arena(42);
    await_first_minion(&mut sim);
    let minions = minion_entities(&mut sim);
    assert_eq!(minions.len(), 1, "exactly one minion from the first whiff-cast");
    let minion = minions[0];

    let world = sim.world_mut();
    let faction = *world.get::<Faction>(minion).expect("minion carries a Faction");
    assert_eq!(faction, Faction::Friendly, "the DK's pet fights for the Friendly side");

    let mut instances = world.query::<&AbilityInstance>();
    assert!(
        instances.iter(world).any(|i| i.owner == minion && i.def_id == "companion_attack"),
        "the minion owns its own companion_attack instance (not the player's Death Strike)"
    );
}

#[test]
fn minion_seeks_and_damages_a_nearby_enemy() {
    let mut sim = Sim::new_arena(7);
    sim.set_player_pos(Vec2::ZERO);
    await_first_minion(&mut sim);
    let player = sim.player();
    let enemy = sim.spawn_grunt((2, 0)); // 64 units — outside its own contact range of the player
    sim.set_health(enemy, 100.0);
    // Root the grunt so it holds still instead of also closing on the player: with both the minion
    // and the grunt converging head-on along the same axis, their relative bearing swings through a
    // near-degenerate (both very close, facing computed a half-step stale relative to position)
    // window every approach, which can whiff the melee_cone's angle check repeatedly. A stationary
    // target keeps this scenario about "does the minion close distance and land its own attack,"
    // not about that geometry edge case.
    sim.apply_status(enemy, player, "root", 1);

    // 45 u/s closes 64 units in ~1.4s; 3s leaves room for travel + at least one 1.5s-cooldown swing.
    sim.step_seconds(3.0);

    assert!(
        sim.enemy_health(enemy).unwrap() < 100.0,
        "the minion chased down and struck the grunt"
    );
}

#[test]
fn minion_expires_after_its_lifetime_and_is_not_immediately_replaced() {
    let mut sim = Sim::new_arena(9);
    await_first_minion(&mut sim);

    // companion_duration is 8s; Companion's own recast cooldown is 10s, so no replacement should
    // have spawned yet either.
    sim.step_seconds(8.5);
    assert_eq!(minion_entities(&mut sim).len(), 0, "the minion despawned after its 8s lifetime");
}

#[test]
fn minion_is_reaped_on_run_restart() {
    let mut sim = Sim::new_arena(3);
    await_first_minion(&mut sim);
    let minions = minion_entities(&mut sim);
    assert_eq!(minions.len(), 1, "a minion exists before the restart");
    let old_minion = minions[0];

    sim.request_start_run("blood_death_knight", 99);
    sim.step(2);

    // The fresh run's own Companion may itself have already whiff-cast a *new* minion within these
    // two frames, so assert the OLD entity specifically is gone rather than "count == 0".
    assert!(!sim.entity_exists(old_minion), "restart tore down the old minion");

    // No orphaned AbilityInstance survives for the despawned minion (the §8.5 orphan-leak class).
    let world = sim.world_mut();
    let mut q = world.query::<&AbilityInstance>();
    assert!(
        q.iter(world).all(|i| i.owner != old_minion),
        "the old minion's own companion_attack instance was reaped too"
    );
}
