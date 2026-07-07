// Golden scenarios — Phase 2 progression & talent flow.
//
// Locks in: XP overflow across multiple levels in one frame, band-pool ability unlocks
// (L2–L6), the AbilityUnlock → TalentChoices phase flip, the TalentPicker state round-trip,
// offer generation respecting uniqueness (Stack / Exclusive) against *current* acquisitions
// (including earlier picks from the same multi-level backlog), decline via Esc, and the
// modifier stack changing real ability output.

use bevy::math::Vec2;
use bevy::prelude::KeyCode;
use rust_game::core::events::GainXpEvent;
use rust_game::game::state::GameState;
use rust_game::progression::state::LevelUpPhase;
use rust_game::sim::Sim;
use rust_game::talent::systems::apply::TalentAcquiredEvent;

/// XP needed to go from level 1 to `level` (linear curve: 10 + (l-1)*5 per level).
fn xp_to_reach(level: u32) -> u32 {
    (1..level).map(|l| 10 + (l - 1) * 5).sum()
}

fn gain_xp(sim: &mut Sim, amount: u32) {
    let player = sim.player();
    sim.world_mut().send_event(GainXpEvent { target: player, amount });
}

fn acquire(sim: &mut Sim, talent_id: &str) {
    let player = sim.player();
    sim.world_mut().send_event(TalentAcquiredEvent {
        owner: player,
        talent_id: talent_id.to_string(),
    });
}

#[test]
fn six_levels_in_one_frame_unlock_all_bands_then_owe_a_choice() {
    let mut sim = Sim::new_arena(11);
    assert_eq!(sim.owned_abilities().len(), 3, "L1: death_strike, dnd, companion");

    // 135 XP = exactly levels 2..=7 in a single frame.
    gain_xp(&mut sim, xp_to_reach(7));
    sim.step(2);

    assert_eq!(sim.player_level(), 7);
    // Five band unlocks (2 from the 2/3 pool + 3 from the 4/6 pool) → 8 abilities total.
    let owned = sim.owned_abilities();
    assert_eq!(owned.len(), 8, "all five band abilities unlocked, got {owned:?}");
    for id in ["blood_boil", "heart_strike", "abomination_limb", "purgatory", "amz"] {
        assert!(owned.contains(&id.to_string()), "{id} unlocked");
    }
    assert_eq!(sim.level_flow().phase, LevelUpPhase::TalentChoices);

    // The 6th level-up owed a talent choice → picker is open with a 3-option offer.
    assert_eq!(sim.game_state(), GameState::TalentPicker);
    let offer = sim.level_flow().pending_offer.clone().expect("offer generated");
    assert_eq!(offer.options.len(), 3, "3 of the 4 loaded death_strike talents offered");

    // Pick option 1 → acquired, backlog drained, back to InRun.
    let picked = offer.options[0].clone();
    sim.tap_key(KeyCode::Digit1);
    sim.step(2);
    assert_eq!(sim.game_state(), GameState::InRun);
    let acquired = sim.acquired_talents();
    assert_eq!(acquired, vec![(picked, 1)], "picked talent landed in AcquiredTalents");
}

#[test]
fn decline_consumes_the_choice_without_acquiring() {
    let mut sim = Sim::new_arena(11);
    gain_xp(&mut sim, xp_to_reach(7));
    sim.step(2);
    assert_eq!(sim.game_state(), GameState::TalentPicker);

    sim.tap_key(KeyCode::Escape);
    sim.step(2);
    assert_eq!(sim.game_state(), GameState::InRun);
    assert!(sim.acquired_talents().is_empty(), "declined offer acquires nothing");
}

#[test]
fn offers_respect_uniqueness_against_current_acquisitions() {
    let mut sim = Sim::new_arena(11);

    // Reach the TalentChoices phase and clear the first owed choice by declining.
    gain_xp(&mut sim, xp_to_reach(7));
    sim.step(2);
    sim.tap_key(KeyCode::Escape);
    sim.step(2);
    assert_eq!(sim.game_state(), GameState::InRun);

    // Exhaust two of the four death_strike talents: the Exclusive epic (one copy) and the
    // damage common at its Stack(3) cap.
    acquire(&mut sim, "death_strike_bone_shield_epic");
    acquire(&mut sim, "death_strike_damage_common");
    acquire(&mut sim, "death_strike_damage_common");
    acquire(&mut sim, "death_strike_damage_common");
    sim.step(1);

    // Owe two more choices (levels 8 and 9 = 40 + 45 XP).
    gain_xp(&mut sim, 85);
    sim.step(2);
    assert_eq!(sim.game_state(), GameState::TalentPicker);

    // Offer 1: the exhausted Exclusive epic and capped Stack(3) common are filtered out. (Not
    // asserting the full eligible set here — it grows as later Phase-9 sub-phases add more BDK
    // content the level-7 band unlocks now also own; see offer2's identical style below.)
    let offer1 = sim.level_flow().pending_offer.clone().expect("offer 1");
    assert!(
        !offer1.options.contains(&"death_strike_bone_shield_epic".to_string())
            && !offer1.options.contains(&"death_strike_damage_common".to_string()),
        "capped/exclusive talents filtered from the offer: {:?}",
        offer1.options
    );

    // Pick one; the next offer (same backlog) must reflect that acquisition immediately —
    // guards the install_acquired_talent → refill_offer ordering.
    sim.tap_key(KeyCode::Digit1);
    sim.step(2);
    assert_eq!(sim.game_state(), GameState::TalentPicker, "second owed choice pending");
    let offer2 = sim.level_flow().pending_offer.clone().expect("offer 2");
    assert!(
        !offer2.options.contains(&"death_strike_bone_shield_epic".to_string())
            && !offer2.options.contains(&"death_strike_damage_common".to_string()),
        "ineligible talents stay excluded in the backlog's second offer: {:?}",
        offer2.options
    );

    sim.tap_key(KeyCode::Digit1);
    sim.step(2);
    assert_eq!(sim.game_state(), GameState::InRun, "backlog drained");
}

#[test]
fn damage_talent_multiplies_death_strike_output() {
    let mut sim = Sim::new_arena(11);
    sim.disable_companion(); // Phase 9.2: isolate Death Strike's own damage from the pet

    // Three copies of +20% damage: 10 * (1 + 0.6) = 16 per hit.
    acquire(&mut sim, "death_strike_damage_common");
    acquire(&mut sim, "death_strike_damage_common");
    acquire(&mut sim, "death_strike_damage_common");
    sim.step(1);

    let brute = sim.spawn_enemy("brute", (1, 1)); // 30 hp, out of contact range
    sim.set_player_facing(Vec2::new(1.0, 1.0));
    sim.trigger_ability("death_strike");
    sim.step(1);

    assert_eq!(
        sim.enemy_health(brute),
        Some(14.0),
        "30 - 16 modified damage (10 * 1.6)"
    );
}

#[test]
fn leech_talent_multiplies_leech_only() {
    let mut sim = Sim::new_arena(11);
    sim.disable_companion(); // Phase 9.2: isolate Death Strike's own damage/leech from the pet
    sim.set_player_health(50.0);

    // Two copies of +20% leech: 5% * 1.4 = 7% of 10 damage = 0.7 healed.
    acquire(&mut sim, "death_strike_leech_common");
    acquire(&mut sim, "death_strike_leech_common");
    sim.step(1);

    let brute = sim.spawn_enemy("brute", (1, 1));
    sim.set_player_facing(Vec2::new(1.0, 1.0));
    sim.trigger_ability("death_strike");
    sim.step(1);

    assert_eq!(sim.enemy_health(brute), Some(20.0), "damage unmodified by the leech talent");
    assert!(
        (sim.player_health() - 50.7).abs() < 1e-3,
        "leech scaled to 0.7, got {}",
        sim.player_health()
    );
}

#[test]
fn band_unlocks_are_seed_deterministic() {
    let mut a = Sim::new_arena(77);
    let mut b = Sim::new_arena(77);
    for sim in [&mut a, &mut b] {
        gain_xp(sim, 10); // one level → one band draw
        sim.step(2);
    }
    assert_eq!(
        a.owned_abilities(),
        b.owned_abilities(),
        "same seed → same band draw order"
    );
}
