// Golden scenarios — movement & collision.
//
// Locks in the prototype's movement behavior: WASD velocity, per-axis wall collision with
// slide-along-wall, and map-boundary blocking. If a refactor changes any of these numbers,
// either it is a regression or the CHANGELOG must declare the change and this test updates.

use bevy::prelude::KeyCode;
use rust_game::sim::{Sim, SIM_DT};

#[test]
fn wasd_moves_player_at_player_speed() {
    let mut sim = Sim::new_arena(42);
    let start = sim.player_pos();

    sim.press_key(KeyCode::KeyD);
    sim.step(61); // one frame of input latency + 60 movement frames
    sim.release_key(KeyCode::KeyD);

    let moved = sim.player_pos().x - start.x;
    // PLAYER_SPEED = 35 world units/sec; allow ±1 frame of slack.
    let per_frame = 35.0 * SIM_DT;
    assert!(
        (moved - 35.0).abs() <= per_frame + 1e-3,
        "expected ~35 units of +X movement in 1s, got {moved}"
    );
    assert_eq!(sim.player_pos().y, start.y, "no Y drift from pure +X input");
}

#[test]
fn wall_blocks_axis_but_player_slides_along_it() {
    let mut sim = Sim::new_arena(42);
    // A solid wall column immediately to the player's right (tile x=1 spans world x≈16..48).
    for y in -3..=3 {
        sim.block_tile(1, y);
    }

    // Push diagonally into the wall: +X blocked, +Y free → the player slides upward.
    sim.press_key(KeyCode::KeyD);
    sim.press_key(KeyCode::KeyW);
    sim.step(120); // 2 seconds
    sim.release_key(KeyCode::KeyD);
    sim.release_key(KeyCode::KeyW);

    let pos = sim.player_pos();
    assert!(
        pos.x < 16.5,
        "X movement stopped at the wall boundary (tile x=1 starts at world 16), got x={}",
        pos.x
    );
    assert!(
        pos.y > 30.0,
        "player slid along the wall on the free Y axis, got y={}",
        pos.y
    );
}

#[test]
fn map_border_is_impassable() {
    let mut sim = Sim::new_arena(42);
    // Teleport near the east border (map half-extent is 40 tiles ≈ world x 1280).
    sim.set_player_pos(bevy::math::Vec2::new(39.0 * 32.0, 0.0));
    sim.step(1);

    sim.press_key(KeyCode::KeyD);
    sim.step(600); // 10 seconds pushing east
    sim.release_key(KeyCode::KeyD);

    let pos = sim.player_pos();
    assert!(
        pos.x < 40.0 * 32.0 - 15.0,
        "player cannot cross the border wall, got x={}",
        pos.x
    );
}
