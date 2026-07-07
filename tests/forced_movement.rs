// Golden scenarios — forced movement (Phase 9.1, §8.1(6)).
//
// `ForcedImpulse` overrides Velocity for its duration, then removes itself; the normal per-axis
// TileMap wall-slide (core/systems/movement.rs) still applies underneath it. No shipped ability
// grants one yet (Abomination Limb's grip lands in Phase 9.2), so this drives the primitive
// directly.

use bevy::math::Vec2;
use rust_game::sim::Sim;

#[test]
fn grip_pulls_an_enemy_toward_the_caster() {
    let mut sim = Sim::new_arena(42);
    let enemy = sim.spawn_grunt((10, 0)); // world x ~= 320, well clear of the origin
    let start = sim.entity_pos(enemy).unwrap();

    sim.pull_toward(enemy, Vec2::ZERO, 200.0, 0.5);
    sim.step(30); // 0.5s of pull

    let pos = sim.entity_pos(enemy).unwrap();
    assert!(pos.x < start.x, "pulled toward the origin: start={start:?} now={pos:?}");
}

#[test]
fn the_impulse_expires_and_stops_moving_the_entity() {
    let mut sim = Sim::new_arena(42);
    let enemy = sim.spawn_grunt((10, 0));

    sim.pull_toward(enemy, Vec2::ZERO, 200.0, 0.2); // 0.2s pull
    sim.step(12); // exactly the impulse duration
    let pos_at_expiry = sim.entity_pos(enemy).unwrap();

    // A grunt this far from the player has no flow-field velocity of its own (out of FLOW_RADIUS
    // reach in this arena... but to be safe just check it doesn't keep moving at pull speed).
    sim.step(6); // 0.1s more
    let pos_after = sim.entity_pos(enemy).unwrap();
    let moved_after_expiry = pos_after.distance(pos_at_expiry);
    assert!(
        moved_after_expiry < 200.0 * 0.1 - 1.0,
        "impulse should have expired and stopped driving the entity at pull speed, moved {moved_after_expiry}"
    );
}

#[test]
fn knockback_pushes_and_stops_at_a_wall() {
    let mut sim = Sim::new_arena(42);
    // A wall immediately to the +X side of a fresh grunt (tile x=1 spans world x ~ 16..48).
    sim.block_tile(1, 0);
    let enemy = sim.spawn_grunt((0, 0));

    sim.knockback(enemy, Vec2::X, 500.0, 1.0);
    sim.step(60); // 1s — enough to travel 500 units if unobstructed

    let pos = sim.entity_pos(enemy).unwrap();
    assert!(
        pos.x < 16.5,
        "knockback stopped at the wall boundary (tile x=1 starts at world 16), got x={}",
        pos.x
    );
}
