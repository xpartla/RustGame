// Golden scenarios — persistent zones (Phase 6).
//
// Locks in: dropped-zone lifecycle (D&D drops a zone that expires), the PlayerZonePresence spatial
// cache (enter/exit), the zone-conditioned ability hook (D&D doubles Blood Boil range inside it),
// generic occupant-tick effects (Consecrated Ground DoT hits the opposing faction only; D&D regen
// heals the owner inside), and AMZ projectile blocking (destroys enemy bolts entering it, except
// those emitted from inside). Follow-anchor tracking is covered alongside AMZ.
//
// Zones carry NO RNG, so none of this touches the golden-master reproducibility guarantee.

use bevy::math::Vec2;
use rust_game::core::components::Faction;
use rust_game::sim::Sim;

// ── 6B: dropped-zone lifecycle + presence ────────────────────────────────────────────────

#[test]
fn dnd_cast_spawns_zone_that_expires() {
    let mut sim = Sim::new_arena(1);
    sim.set_player_pos(Vec2::ZERO);
    assert_eq!(sim.zone_count(), 0, "no zones at start");

    // D&D is the DK's RMB Special (activation Input); trigger it directly.
    sim.trigger_ability("dnd");
    // Frame 1: execute spawns the zone. Frame 2: presence (MovementSet::Integrate) picks it up
    // (the zone is spawned in the later CombatSet::Damage, so presence lags one frame by design).
    sim.step(2);

    assert_eq!(sim.zone_count(), 1, "D&D dropped exactly one zone");
    assert_eq!(sim.zone_types(), vec!["death_and_decay".to_string()]);
    assert_eq!(sim.zone_center("death_and_decay"), Some(Vec2::ZERO), "dropped at the caster");
    assert!(sim.player_in_zone("death_and_decay"), "player stands in the fresh zone");

    // zone_duration = 8s; step just past it and tick_zone_lifetimes reaps it.
    sim.step_seconds(8.5);
    assert_eq!(sim.zone_count(), 0, "zone expired and was despawned");
    assert!(!sim.player_in_zone("death_and_decay"), "presence clears once the zone is gone");
}

#[test]
fn presence_tracks_enter_and_exit() {
    let mut sim = Sim::new_arena(2);
    sim.set_player_pos(Vec2::ZERO);
    sim.trigger_ability("dnd"); // fixed zone at origin, radius 80
    sim.step(2);
    assert!(sim.player_in_zone("death_and_decay"), "inside the fresh zone");

    // Walk well beyond the 80-unit radius.
    sim.set_player_pos(Vec2::new(200.0, 0.0));
    sim.step(1);
    assert!(!sim.player_in_zone("death_and_decay"), "left the zone → presence clears");

    // Return.
    sim.set_player_pos(Vec2::ZERO);
    sim.step(1);
    assert!(sim.player_in_zone("death_and_decay"), "re-entered → presence set again");
}

// ── 6C: the zone-conditioned ability hook (testing.md Phase-6 DoD) ────────────────────────

#[test]
fn dnd_doubles_blood_boil_range_inside() {
    // A durable dummy 128 units away — OUTSIDE Blood Boil's 90 radius, INSIDE its D&D-doubled 180.
    // The ONLY difference between the two runs is whether the caster stands in D&D; the
    // blood_boil_dnd_range Pre hook (installed by the talent) doubles `radius` only when inside.
    let dummy_tile = (4, 0); // x = 4 * 32 = 128

    // --- control: no D&D → Blood Boil can't reach the dummy ---
    let mut ctrl = Sim::new_arena(7);
    ctrl.set_player_pos(Vec2::ZERO);
    let d1 = ctrl.spawn_grunt(dummy_tile);
    ctrl.set_health(d1, 1000.0); // durable, so a hit shows as a health drop rather than a kill
    ctrl.grant_ability("blood_boil"); // radius 90, auto-casts on cooldown
    ctrl.grant_talent("blood_boil_dnd_range_rare"); // installs the Pre hook (inert outside D&D)
    ctrl.step(20); // Blood Boil auto-casts; 128 > 90 ⇒ miss
    assert!(!ctrl.player_in_zone("death_and_decay"));
    assert_eq!(ctrl.enemy_health(d1), Some(1000.0), "Blood Boil can't reach 128 without D&D");

    // --- treatment: standing in D&D → the hook doubles radius → the same dummy is hit ---
    let mut trt = Sim::new_arena(7);
    trt.set_player_pos(Vec2::ZERO);
    let d2 = trt.spawn_grunt(dummy_tile);
    trt.set_health(d2, 1000.0);
    trt.grant_ability("blood_boil");
    trt.grant_talent("blood_boil_dnd_range_rare");
    trt.trigger_ability("dnd"); // drop D&D at origin (radius 80) — the player stands inside
    trt.step(20);
    assert!(trt.player_in_zone("death_and_decay"), "player is inside D&D");
    assert!(
        trt.enemy_health(d2).unwrap() < 1000.0,
        "inside D&D the Pre hook doubled Blood Boil's radius (90→180) → the 128-unit dummy is hit",
    );
}

// ── 6D: generic zone occupant-tick effects ───────────────────────────────────────────────

#[test]
fn consecrated_ground_dot_damages_opposing_faction_only() {
    let mut sim = Sim::new_arena(3);
    sim.set_player_pos(Vec2::ZERO);
    // Consecrated Ground: an AutoCast dropped_zone with a Holy DoT (radius 60, dps 3).
    sim.grant_ability("consecrated_ground");
    let inside = sim.spawn_grunt((1, 0)); // x = 32 (inside the 60 radius)
    let outside = sim.spawn_grunt((10, 0)); // x = 320 (far outside)
    sim.set_health(inside, 1000.0); // durable so the tick shows as a health drop, not a kill
    sim.set_health(outside, 1000.0);
    // Stun both (immobilize + suppress, 1.5s) so they neither chase nor contact-attack the player —
    // isolating the ZONE's damage from enemy contact. Incoming damage still applies to them.
    let src = sim.player();
    sim.apply_status(inside, src, "stun", 1);
    sim.apply_status(outside, src, "stun", 1);

    // ~70 frames: the zone drops, and one 1 Hz tick lands (the DoT ticks once alive a full second).
    sim.step(70);

    assert!(
        sim.enemy_health(inside).unwrap() < 1000.0,
        "an enemy inside Consecrated Ground took the Holy DoT",
    );
    assert_eq!(sim.enemy_health(outside), Some(1000.0), "an enemy outside took nothing");
    // The zone is Friendly (player-owned) → the player (Friendly) inside is unharmed by its own DoT
    // (and the stunned enemies never reached the player), so it stays at full health.
    assert_eq!(sim.player_health(), 100.0, "the owner's own zone never damages the owner's side");
}

#[test]
fn dnd_regen_heals_owner_inside() {
    let mut sim = Sim::new_arena(4);
    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_health(50.0); // below max (100) so regen is visible
    sim.trigger_ability("dnd"); // D&D at origin: regen 0.5%/s of max (100) = 0.5 hp per tick
    sim.step(70); // one tick while standing inside
    assert!(sim.player_in_zone("death_and_decay"));
    let healed = sim.player_health();
    assert!(healed > 50.0, "D&D regen healed the owner standing inside (was 50, now {healed})");

    // Step outside the zone → regen stops.
    sim.set_player_pos(Vec2::new(300.0, 0.0));
    sim.step(1); // presence updates
    let before = sim.player_health();
    sim.step(70);
    assert_eq!(sim.player_health(), before, "no regen once the owner leaves the zone");
}

// ── 6E: AMZ projectile blocking + the follow anchor ──────────────────────────────────────

#[test]
fn amz_blocks_enemy_projectile() {
    // Control: no AMZ → the spitter's bolt reaches and damages the player.
    let mut ctrl = Sim::new_arena(8);
    ctrl.set_player_pos(Vec2::ZERO);
    ctrl.spawn_enemy("spitter", (4, 0)); // x = 128 (inside its 140 preferred_range → stops + fires)
    let hp0 = ctrl.player_health();
    ctrl.step_seconds(4.0);
    assert!(ctrl.player_health() < hp0, "without AMZ the spitter bolt hits the player");

    // Treatment: a Friendly AMZ (radius 90) around the player destroys the bolt before it lands.
    let mut trt = Sim::new_arena(8);
    trt.set_player_pos(Vec2::ZERO);
    trt.grant_ability("amz"); // auto-casts a Friendly AMZ zone at the player
    trt.spawn_enemy("spitter", (4, 0)); // fires from x = 128, OUTSIDE the AMZ → its bolt is blocked
    let hp0 = trt.player_health();
    trt.step_seconds(4.0);
    assert!(trt.player_in_zone("amz"), "AMZ is up around the player");
    assert_eq!(trt.player_health(), hp0, "AMZ destroyed the enemy bolt → the player is unharmed");
}

#[test]
fn amz_ignores_projectile_emitted_from_inside() {
    // Mechanics: "if enemies emit projectiles from inside the zone it has no effect."
    let mut sim = Sim::new_arena(9);
    sim.set_player_pos(Vec2::ZERO);
    sim.grant_ability("amz"); // AMZ radius 90 at the player
    sim.spawn_enemy("spitter", (2, 0)); // x = 64, INSIDE the AMZ → its bolts are emitted from inside
    let hp0 = sim.player_health();
    sim.step_seconds(4.0);
    assert!(sim.player_in_zone("amz"));
    assert!(
        sim.player_health() < hp0,
        "a bolt emitted from inside the AMZ is not blocked → it still hits the player",
    );
}

#[test]
fn follow_anchor_zone_tracks_owner() {
    // The AMZ-epic "attached to you" mechanism (ZoneAnchor::Follow): the zone centre tracks the owner.
    let mut sim = Sim::new_arena(10);
    sim.set_player_pos(Vec2::ZERO);
    let player = sim.player();
    sim.spawn_zone("amz", Vec2::ZERO, 90.0, 30.0, Some(player), Faction::Friendly);
    sim.step(2);
    assert!(sim.player_in_zone("amz"), "inside the follow zone at the start");

    // Move the player far away: a Fixed zone would be left behind; a Follow zone tracks.
    sim.set_player_pos(Vec2::new(400.0, 0.0));
    sim.step(2);
    assert_eq!(sim.zone_center("amz"), Some(Vec2::new(400.0, 0.0)), "zone centre tracked the owner");
    assert!(sim.player_in_zone("amz"), "still inside after the owner moved");
}
