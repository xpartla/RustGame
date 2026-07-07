// Golden scenarios — status effects (Phase 3).
//
// Locks in the status lifecycle over the sim harness: application, DoT ticking cadence,
// RefreshOnReapply single-instance semantics, and expiry. Tuning is read from the
// assets/status_effects/*.status.ron files (bleed: 1.0s tick / 3 dmg / 4.0s duration) — changing
// those values intentionally will fail these assertions; update them in the same change.
//
// Timing model (see docs/phase3-plan.md §2.6): a bleed instance applied on frame F starts a
// Repeating 1.0s tick timer; ticks fire at ~1/2/3/4 s (frames ~60/120/180/240) and the DoT
// DamageEvent lands one frame later. Duration expires at ~4.0 s. The 3.5 s / 5.0 s probe points
// sit safely between tick boundaries so the assertions are float-robust.

use bevy::math::Vec2;
use rust_game::core::events::DamageTag;
use rust_game::player::components::Experience;
use rust_game::sim::Sim;
use rust_game::status::assets::{StackingRule, StatusEffectDef};

/// A minimal synthetic def for stacking rules no shipped effect uses yet.
fn synthetic_def(id: &str, stacking: StackingRule) -> StatusEffectDef {
    StatusEffectDef {
        id: id.to_string(),
        display_name: id.to_string(),
        stacking,
        base_duration_secs: 10.0,
        tick: None,
        move_speed_mult: 1.0,
        damage_taken_mult: 1.0,
        immobilize: false,
        suppress_abilities: false,
        removed_by_tags: Vec::new(),
        removes_on_apply: Vec::new(),
        hooks: Vec::new(),
    }
}

#[test]
fn bleed_applies_ticks_on_cadence_and_expires() {
    let mut sim = Sim::new_arena(42);
    sim.disable_companion(); // Phase 9.2: isolate the bleed DoT from the DK's pet (long-running test)
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));
    sim.set_health(enemy, 100.0); // durable dummy: survives the whole DoT

    sim.apply_status(enemy, player, "bleed", 1);
    sim.step(1); // apply_status_effects spawns the instance this frame
    assert!(sim.has_status(enemy, "bleed"), "bleed applied");

    // 3.5 s in: exactly three 1-second ticks have landed (3 × 3 = 9 damage).
    sim.step(209); // total 210 frames = 3.5 s
    assert_eq!(sim.enemy_health(enemy), Some(91.0), "3 bleed ticks landed by 3.5s");
    assert!(sim.has_status(enemy, "bleed"), "still active mid-duration");

    // 5.0 s in: the 4th tick landed at ~4 s and the effect expired (4.0 s duration).
    sim.step(90); // total 300 frames = 5.0 s
    assert_eq!(sim.enemy_health(enemy), Some(88.0), "4 ticks total, then no more");
    assert!(!sim.has_status(enemy, "bleed"), "bleed expired after its 4.0s duration");
}

#[test]
fn bleed_refresh_on_reapply_keeps_single_instance_and_extends() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));
    sim.set_health(enemy, 200.0);

    sim.apply_status(enemy, player, "bleed", 1);
    sim.step(181); // ~3.0 s into the first application (would expire at 4.0 s)
    assert_eq!(sim.status_ids_on(enemy).len(), 1, "one instance");

    // Re-apply at ~3 s: RefreshOnReapply must NOT stack a second instance, and must reset the
    // duration (new expiry ~7 s instead of the original ~4 s).
    sim.apply_status(enemy, player, "bleed", 1);
    sim.step(1);
    assert_eq!(sim.status_ids_on(enemy).len(), 1, "still one instance after re-apply");

    sim.step(180); // ~6 s total — past the original 4 s expiry, before the refreshed 7 s
    assert!(sim.has_status(enemy, "bleed"), "refresh extended the duration");
}

#[test]
fn same_frame_double_apply_keeps_a_single_refresh_instance() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));
    sim.set_health(enemy, 100.0);

    // Two applications land in the SAME frame (e.g. two projectiles impacting together).
    // RefreshOnReapply must still yield exactly one instance — the spawn queued by the first
    // event is not yet visible to the second, so this needs explicit same-frame bookkeeping.
    sim.apply_status(enemy, player, "bleed", 1);
    sim.apply_status(enemy, player, "bleed", 1);
    sim.step(1);
    assert_eq!(sim.status_ids_on(enemy).len(), 1, "one bleed instance, not two");

    // And the DoT is single: exactly one 3-damage tick by 1.5 s.
    sim.step(89);
    assert_eq!(sim.enemy_health(enemy), Some(97.0), "no doubled ticking");
}

#[test]
fn stack_capped_status_respects_its_cap_within_and_across_frames() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));
    sim.insert_status_def(synthetic_def("test_capped", StackingRule::StackCapped(3)));

    // Five same-frame applications → capped at three (pending spawns count toward the cap).
    for _ in 0..5 {
        sim.apply_status(enemy, player, "test_capped", 1);
    }
    sim.step(1);
    assert_eq!(sim.status_ids_on(enemy).len(), 3, "cap holds within one frame");

    // A later application is still rejected while the cap is full.
    sim.apply_status(enemy, player, "test_capped", 1);
    sim.step(1);
    assert_eq!(sim.status_ids_on(enemy).len(), 3, "cap holds across frames");
}

#[test]
fn stack_unlimited_spawns_one_instance_per_stack() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));
    sim.insert_status_def(synthetic_def("test_unlimited", StackingRule::StackUnlimited));

    sim.apply_status(enemy, player, "test_unlimited", 2); // one event, two stacks
    sim.apply_status(enemy, player, "test_unlimited", 1); // same frame, one more
    sim.step(1);
    assert_eq!(sim.status_ids_on(enemy).len(), 3, "2 + 1 stacks, all live");
}

#[test]
fn statuses_are_reaped_when_their_target_dies() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));
    sim.apply_status(enemy, player, "bleed", 1);
    sim.step(1);
    assert_eq!(sim.active_status_count(), 1, "bleed instance live");

    sim.deal_damage(enemy, 999.0);
    sim.step(2); // death, then the orphan sweep reaps the instance
    assert_eq!(sim.enemy_health(enemy), None, "target died");
    assert_eq!(sim.active_status_count(), 0, "no orphaned status instances");
}

#[test]
fn unknown_status_id_is_ignored() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));

    // An id with no loaded StatusEffectDef self-filters (like unregistered ability behaviors).
    sim.apply_status(enemy, player, "no_such_effect", 1);
    sim.step(2);
    assert!(sim.status_ids_on(enemy).is_empty(), "unknown status id applies nothing");
}

// ── CC & stat integration (Phase 3C) ────────────────────────────────────────────────────

#[test]
fn frostbite_slows_enemy_movement() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    sim.set_player_pos(Vec2::ZERO);
    // Symmetric approach: both grunts chase the origin from equal distance, so absent frostbite
    // their displacement is identical. Frostbite multiplies only the integration step (0.8).
    let frosted = sim.spawn_grunt((8, 0));
    let control = sim.spawn_grunt((-8, 0));
    sim.step(1);
    sim.apply_status(frosted, player, "frostbite", 1);
    sim.step(2); // frostbite instance + MoveSpeedModifier resolved

    let start_f = sim.entity_pos(frosted).unwrap();
    let start_c = sim.entity_pos(control).unwrap();
    sim.step(120); // 2 s of chasing
    let d_f = (sim.entity_pos(frosted).unwrap() - start_f).length();
    let d_c = (sim.entity_pos(control).unwrap() - start_c).length();

    assert!(d_c > 1.0, "control grunt actually moved ({d_c})");
    assert!(d_f < d_c, "frostbite slowed the frosted grunt ({d_f} < {d_c})");
    let ratio = d_f / d_c;
    assert!((0.72..0.88).contains(&ratio), "≈0.8× move speed, got {ratio}");
}

#[test]
fn frostbite_amplifies_damage_taken() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));
    sim.apply_status(enemy, player, "frostbite", 1);
    sim.step(2); // DamageTakenModifier (1.1) resolved

    sim.set_health(enemy, 100.0);
    sim.deal_damage(enemy, 10.0); // untagged — does not clear frostbite
    sim.step(1);
    assert_eq!(sim.enemy_health(enemy), Some(89.0), "10 damage × 1.1 frostbite amp = 11");
}

#[test]
fn root_immobilizes_then_releases() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    sim.set_player_pos(Vec2::ZERO);
    let enemy = sim.spawn_grunt((8, 0));

    sim.step(40); // moving toward the player

    sim.apply_status(enemy, player, "root", 1);
    sim.step(2); // Immobilized resolved
    let rooted_at = sim.entity_pos(enemy).unwrap();
    sim.step(90); // 1.5 s of root (duration 2.5 s)
    let still = sim.entity_pos(enemy).unwrap();
    assert!((still - rooted_at).length() < 0.01, "rooted enemy is frozen");

    // Past the 2.5 s root: it moves again.
    sim.step(90); // total root age ~3.0 s → expired
    let after = sim.entity_pos(enemy).unwrap();
    assert!((after - still).length() > 1.0, "movement resumes after root expires");
}

#[test]
fn stun_immobilizes_enemy() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    sim.set_player_pos(Vec2::ZERO);
    let enemy = sim.spawn_grunt((8, 0));
    sim.step(40);

    sim.apply_status(enemy, player, "stun", 1);
    sim.step(2);
    let stunned_at = sim.entity_pos(enemy).unwrap();
    sim.step(60); // 1.0 s of stun (duration 1.5 s)
    let still = sim.entity_pos(enemy).unwrap();
    assert!((still - stunned_at).length() < 0.01, "stunned enemy is frozen");
}

#[test]
fn fire_tagged_damage_removes_frostbite() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));
    sim.set_health(enemy, 100.0);

    sim.apply_status(enemy, player, "frostbite", 1);
    sim.step(2);
    assert!(sim.has_status(enemy, "frostbite"), "frostbite applied");

    sim.deal_tagged_damage(enemy, 1.0, DamageTag::Fire);
    sim.step(2);
    assert!(!sim.has_status(enemy, "frostbite"), "fire damage cleared frostbite");
}

#[test]
fn frost_tagged_damage_removes_blaze() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));
    sim.set_health(enemy, 100.0);

    sim.apply_status(enemy, player, "blaze", 1);
    sim.step(2);
    assert!(sim.has_status(enemy, "blaze"), "blaze applied");

    sim.deal_tagged_damage(enemy, 1.0, DamageTag::Frost);
    sim.step(2);
    assert!(!sim.has_status(enemy, "blaze"), "frost damage cleared blaze");
}

#[test]
fn blaze_fire_tick_clears_frostbite_emergently() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((5, 0));
    sim.set_health(enemy, 200.0);

    sim.apply_status(enemy, player, "blaze", 1);
    sim.apply_status(enemy, player, "frostbite", 1);
    sim.step(2);
    assert!(sim.has_status(enemy, "blaze") && sim.has_status(enemy, "frostbite"), "both applied");

    // Blaze's first Fire-tagged tick lands at ~1 s and clears frostbite — no special case.
    sim.step(75); // ~1.25 s
    assert!(!sim.has_status(enemy, "frostbite"), "blaze's fire tick cleared frostbite");
    assert!(sim.has_status(enemy, "blaze"), "blaze itself persists");
}

#[test]
fn dot_kill_credits_the_applier() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    let enemy = sim.spawn_grunt((6, 0));
    sim.set_health(enemy, 5.0); // dies to the second bleed tick (2 × 3 = 6)

    let xp_before = {
        let p = sim.player();
        sim.world().get::<Experience>(p).unwrap().current
    };

    sim.apply_status(enemy, player, "bleed", 1);
    sim.step_seconds(2.5); // two ticks land → enemy dies to the DoT
    assert_eq!(sim.enemy_health(enemy), None, "enemy killed by the bleed DoT");

    let xp_after = {
        let p = sim.player();
        sim.world().get::<Experience>(p).unwrap().current
    };
    assert!(xp_after > xp_before, "the bleed's source (player) got kill XP: {xp_before} → {xp_after}");
}
