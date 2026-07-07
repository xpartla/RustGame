// Golden scenarios — AMZ's talent tree (Phase 9.2).
//
// Locks in: amz_size_common scales zone_radius, amz_duration_common extends zone_duration,
// amz_regen_rare turns on the existing D&D-style occupant regen, amz_movespeed_rare grants
// ZoneSpeedModifier while standing inside, and amz_follow_epic switches the zone's anchor to
// Follow the caster.

use bevy::math::Vec2;
use rust_game::core::components::ZoneSpeedModifier;
use rust_game::sim::Sim;
use rust_game::zone::components::{PersistentZone, ZoneAnchor};

fn find_amz(sim: &mut Sim) -> Option<(f32, f32)> {
    let world = sim.world_mut();
    let mut q = world.query::<&PersistentZone>();
    q.iter(world).find(|z| z.zone_type == "amz").map(|z| (z.radius, z.duration.duration().as_secs_f32()))
}

#[test]
fn size_common_scales_zone_radius() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.grant_talent("amz_size_common");
    sim.grant_ability("amz");
    sim.step(2);

    let (radius, _) = find_amz(&mut sim).expect("amz zone spawned");
    assert!((radius - 108.0).abs() < 1e-3, "90 * 1.2 = 108, got {radius}");
}

#[test]
fn duration_common_extends_zone_duration() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.grant_talent("amz_duration_common");
    sim.grant_ability("amz");
    sim.step(2);

    let (_, duration) = find_amz(&mut sim).expect("amz zone spawned");
    assert!((duration - 8.0).abs() < 1e-3, "6 + 2 = 8, got {duration}");
}

#[test]
fn regen_rare_heals_the_player_while_standing_inside() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.grant_talent("amz_regen_rare");
    sim.grant_ability("amz");
    sim.set_player_health(100.0);
    sim.step(3); // zone spawns centered on the player; presence snapshot trails by one more frame

    assert!(sim.player_in_zone("amz"), "player starts inside their own zone");
    sim.step_seconds(1.1); // past one ZONE_TICK_INTERVAL

    assert!(sim.player_health() > 100.0, "regen_percent_per_second healed the player, got {}", sim.player_health());
}

#[test]
fn movespeed_rare_grants_zonespeedmodifier_while_inside() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.grant_talent("amz_movespeed_rare");
    sim.grant_ability("amz");
    sim.step(3); // zone spawns; presence snapshot + speed-bonus resolve need a frame each

    let player = sim.player();
    let modifier = sim.world().get::<ZoneSpeedModifier>(player).map(|m| m.0);
    assert_eq!(modifier, Some(1.2), "20% speed bonus applied while inside own AMZ");
}

#[test]
fn follow_epic_anchors_the_zone_to_the_caster() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion();
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_talent("amz_follow_epic");
    sim.grant_ability("amz");
    sim.step(2);

    let world = sim.world_mut();
    let mut q = world.query::<&PersistentZone>();
    let anchor_kind = q.iter(world).find(|z| z.zone_type == "amz").map(|z| matches!(z.anchor, ZoneAnchor::Follow(_)));
    assert_eq!(anchor_kind, Some(true), "follow_caster override switches Fixed -> Follow");
}
