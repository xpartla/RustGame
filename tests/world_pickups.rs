// Golden scenarios — map generation determinism & pickups.

use bevy::math::Vec2;
use rust_game::core::components::WorldPosition;
use rust_game::pickup::components::{PickUp, PickUpKind};
use rust_game::sim::Sim;

#[test]
fn map_generation_is_seed_deterministic() {
    let a = Sim::new(7);
    let b = Sim::new(7);
    assert_eq!(
        a.tilemap_signature(),
        b.tilemap_signature(),
        "same seed → identical map"
    );

    let c = Sim::new(8);
    assert_ne!(
        a.tilemap_signature(),
        c.tilemap_signature(),
        "different seed → different map"
    );
}

#[test]
fn spawn_area_is_kept_clear() {
    let sim = Sim::new(7);
    let map = sim.world().resource::<rust_game::world::components::TileMap>();
    for x in -6..=6 {
        for y in -6..=6 {
            assert!(
                !map.blocked.contains(&rust_game::core::components::GridPosition { x, y }),
                "spawn-clear box contains obstacle at ({x},{y})"
            );
        }
    }
}

#[test]
fn heal_pickup_heals_and_clamps_to_max() {
    let mut sim = Sim::new_arena(42);

    // Exact heal below max.
    sim.set_player_health(50.0);
    let pos = sim.player_pos();
    sim.world_mut().spawn((PickUp { kind: PickUpKind::Heal(25.0) }, WorldPosition(pos)));
    sim.step(2);
    assert_eq!(sim.player_health(), 75.0, "heal pack restores 25");

    // Overheal clamps to max (100).
    sim.set_player_health(90.0);
    let pos = sim.player_pos();
    sim.world_mut().spawn((PickUp { kind: PickUpKind::Heal(25.0) }, WorldPosition(pos)));
    sim.step(2);
    assert_eq!(sim.player_health(), 100.0, "heal clamps at max health");
}

#[test]
fn pickup_outside_radius_is_not_collected() {
    let mut sim = Sim::new_arena(42);
    sim.set_player_health(50.0);
    let pos = sim.player_pos() + Vec2::new(48.0, 0.0); // beyond the 24-unit pickup radius
    sim.world_mut().spawn((PickUp { kind: PickUpKind::Heal(25.0) }, WorldPosition(pos)));
    sim.step(5);
    assert_eq!(sim.player_health(), 50.0, "distant pickup not collected");
}
