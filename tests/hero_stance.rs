// Golden scenarios — hero indirection + stance system (Phase 4).
//
// Locks in the mechanic behind the second class:
//   - the input slot resolves through HeroDef.stance_slots + ActiveStance (LMB → the active
//     stance's Basic ability), so the Death Knight still casts Death Strike and the Mage casts
//     Fireblast/Frostbolt per stance;
//   - Q swaps the Mage's stance, remapping LMB to the other element, and applies the entered
//     stance's swap effect (Ice Barrier / Boots of Fire) to the caster;
//   - Q is a no-op for non-stance heroes (Death Knight).
//
// Tuning is read from the RON assets: death_strike 10 dmg (death_strike.ability.ron); fireblast
// 8 Fire dmg + blaze, frostbolt 6 Frost dmg + frostbite (fireblast/frostbolt.ability.ron);
// ice_barrier damage_taken_mult 0.6 (ice_barrier.status.ron).

use bevy::math::Vec2;
use bevy::prelude::{KeyCode, MouseButton};
use rust_game::sim::Sim;

/// The hero indirection preserves the prototype's Death Knight behavior: LMB still casts Death
/// Strike (regression guard for replacing the Phase-1 hardcoded input stub).
#[test]
fn default_death_knight_lmb_casts_death_strike() {
    let mut sim = Sim::new_arena(42);
    assert_eq!(sim.hero_id(), "blood_death_knight");
    assert_eq!(sim.active_stance(), "default");

    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let enemy = sim.spawn_grunt((1, 0)); // 32 units ahead, inside Death Strike's 60-range cone
    sim.set_health(enemy, 100.0);

    sim.tap_mouse(MouseButton::Left); // LMB → Basic slot → death_strike (instant melee cone)
    assert_eq!(sim.enemy_health(enemy), Some(90.0), "LMB cast Death Strike (10 dmg) via the hero layer");
}

/// The second class fires through the same input slots: the Mage's LMB casts the active stance's
/// Basic ability (Fireblast in Fire form).
#[test]
fn second_class_basic_attack_fires_through_input_slots() {
    let mut sim = Sim::new_arena(42);
    sim.set_player_pos(Vec2::ZERO);
    let player = sim.player();
    sim.set_hero(player, "mage", "fire");
    sim.step(1); // deferred grant re-runs → fireblast + frostbolt owned

    let owned = sim.owned_abilities();
    assert!(owned.contains(&"fireblast".to_string()), "Mage owns Fireblast");
    assert!(owned.contains(&"frostbolt".to_string()), "Mage owns Frostbolt");

    sim.set_player_facing(Vec2::X);
    sim.set_health(player, 100_000.0); // survive the closing grunt for the whole test
    let enemy = sim.spawn_grunt((8, 0));
    sim.set_health(enemy, 100.0);

    sim.tap_mouse(MouseButton::Left); // LMB in Fire → fireblast (projectile)
    sim.step(65); // projectile travels + impacts (before blaze's first DoT tick)
    assert_eq!(sim.enemy_health(enemy), Some(92.0), "Mage LMB fired Fireblast (8 dmg) via the Basic slot");
}

/// Q swaps the Mage's stance, so the *same* LMB fires a different element: blaze in Fire, frostbite
/// in Ice. This is the headline "stance swap remaps LMB" scenario.
#[test]
fn stance_swap_remaps_lmb() {
    let mut sim = Sim::new_arena(42);
    sim.set_player_pos(Vec2::ZERO);
    let player = sim.player();
    sim.set_hero(player, "mage", "fire");
    sim.step(1);
    sim.set_player_facing(Vec2::X);
    sim.set_health(player, 100_000.0);

    let enemy = sim.spawn_grunt((8, 0));
    sim.set_health(enemy, 500.0); // survive both hits

    // Fire stance: LMB → Fireblast → blaze (Fire), no frostbite.
    sim.tap_mouse(MouseButton::Left);
    sim.step(65);
    assert!(sim.has_status(enemy, "blaze"), "Fire-stance LMB cast Fireblast → blaze");
    assert!(!sim.has_status(enemy, "frostbite"), "no frostbite while in Fire stance");

    // Swap to Ice.
    sim.tap_key(KeyCode::KeyQ);
    assert_eq!(sim.active_stance(), "ice", "Q swapped Fire → Ice");

    // Ice stance: the same LMB now casts Frostbolt → frostbite, and (being Frost) clears blaze.
    sim.tap_mouse(MouseButton::Left);
    sim.step(65);
    assert!(sim.has_status(enemy, "frostbite"), "Ice-stance LMB cast Frostbolt → frostbite");
    assert!(!sim.has_status(enemy, "blaze"), "Frostbolt (Frost) cleared blaze on impact");
}

/// Entering a stance applies that stance's swap effect to the caster: Ice Barrier (damage
/// reduction) on Fire → Ice, Boots of Fire (move-speed buff) on Ice → Fire.
#[test]
fn stance_swap_applies_entering_stance_effect() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    sim.set_hero(player, "mage", "fire");
    sim.step(1);

    // Fire → Ice grants Ice Barrier.
    sim.tap_key(KeyCode::KeyQ);
    sim.step(1); // apply_status + resolve fold the effect into the actor modifiers
    assert_eq!(sim.active_stance(), "ice");
    assert!(sim.has_status(player, "ice_barrier"), "entering Ice granted Ice Barrier");

    // Ice Barrier reduces incoming damage ×0.6 (damage_taken_mult).
    sim.set_health(player, 100.0);
    sim.deal_damage(player, 50.0);
    sim.step(1);
    assert!(
        (sim.player_health() - 70.0).abs() < 1e-3,
        "Ice Barrier mitigated 50 → 30 (×0.6), got {}",
        sim.player_health()
    );

    // Ice → Fire grants Boots of Fire.
    sim.tap_key(KeyCode::KeyQ);
    sim.step(1);
    assert_eq!(sim.active_stance(), "fire");
    assert!(sim.has_status(player, "boots_of_fire"), "entering Fire granted Boots of Fire");
}

/// Q does nothing for a non-stance hero (Death Knight has_stance == false): no stance change, no
/// swap effect.
#[test]
fn non_stance_hero_q_is_a_noop() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    assert_eq!(sim.hero_id(), "blood_death_knight");
    assert_eq!(sim.active_stance(), "default");

    sim.tap_key(KeyCode::KeyQ);
    sim.step(1);
    assert_eq!(sim.active_stance(), "default", "Death Knight has no stance — Q is a no-op");
    assert!(sim.status_ids_on(player).is_empty(), "no swap effect applied");
}

/// The Movement slot (Shift/Space, Phase 9.1) fires whatever ability is bound to it — no shipped
/// hero claims the slot yet, so this binds the "dash" demonstrator (assets/abilities/dash.ability.ron)
/// to prove the Shift/Space input path reaches TriggerAbilityEvent end-to-end, distinct from
/// ability/behavior.rs's unit test of the `blink` behavior's pure targeting logic.
#[test]
fn movement_slot_triggers_a_dash() {
    let mut sim = Sim::new_arena(42);
    sim.bind_movement_ability("blood_death_knight", "dash");
    sim.grant_ability("dash");
    sim.step(1); // spawn_unlocked_ability

    sim.set_player_pos(Vec2::ZERO);
    sim.set_player_facing(Vec2::X);
    let start = sim.player_pos();

    sim.tap_key(KeyCode::ShiftLeft);
    sim.step(10); // the ForcedImpulse takes effect the frame after the cast (deferred Commands)

    let moved = sim.player_pos().x - start.x;
    assert!(moved > 50.0, "Shift triggered the bound dash (~75 units over 0.15s), moved {moved}");
}

/// The debug playtest hotkey (M) re-identifies the live player as the Mage and grants its kit.
#[test]
fn debug_hotkey_switches_player_to_mage() {
    let mut sim = Sim::new_arena(42);
    assert_eq!(sim.hero_id(), "blood_death_knight");

    sim.tap_key(KeyCode::KeyM); // debug: become the Mage
    sim.step(1); // deferred grant re-runs for the new class
    assert_eq!(sim.hero_id(), "mage");
    assert_eq!(sim.active_stance(), "fire");
    let owned = sim.owned_abilities();
    assert!(owned.contains(&"fireblast".to_string()), "Fireblast granted after the swap");
    assert!(owned.contains(&"frostbolt".to_string()), "Frostbolt granted after the swap");
}
