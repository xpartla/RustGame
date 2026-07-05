// Golden scenarios — merchant operations (Phase 7.5E, D2). The overlay is presentation-only; these
// drive the ops logic (remove-talent, 3-for-1 trade-up) through the real request events, which is
// exactly what the merchant overlay's input emits.

use rust_game::game::state::GameState;
use rust_game::sim::Sim;

/// Removing a Behavior talent at the merchant pops both the talent and its installed ActiveHook
/// (through the Phase-2 `uninstall_removed_talent` consumer).
#[test]
fn merchant_remove_uninstalls_talent_and_hook() {
    let mut sim = Sim::new_arena(3);

    // Acquire a Behavior talent → installs its ActiveHook.
    sim.grant_talent("blood_boil_dnd_range_rare");
    sim.step(1);
    assert!(
        sim.acquired_talents().iter().any(|(id, _)| id == "blood_boil_dnd_range_rare"),
        "talent acquired"
    );
    assert!(sim.active_hooks().iter().any(|h| h == "blood_boil_dnd_range"), "hook installed");

    // Remove it via the merchant op.
    sim.merchant_remove("blood_boil_dnd_range_rare");
    sim.step(2);
    assert!(
        !sim.acquired_talents().iter().any(|(id, _)| id == "blood_boil_dnd_range_rare"),
        "talent removed"
    );
    assert!(
        !sim.active_hooks().iter().any(|h| h == "blood_boil_dnd_range"),
        "hook popped once the last copy is gone"
    );
}

/// A 3-for-1 trade sacrifices three talents and opens a picker floored one rarity above the highest
/// sacrificed — three Commons ⇒ a Rare-or-above offer.
#[test]
fn merchant_trade_offers_higher_rarity() {
    let mut sim = Sim::new_arena(3);

    let commons =
        ["death_strike_leech_common", "death_strike_range_common", "death_strike_damage_common"];
    for id in commons {
        sim.grant_talent(id);
        sim.step(1);
    }
    assert_eq!(sim.acquired_talents().len(), 3, "three Common talents acquired");

    sim.merchant_trade(commons);
    sim.step(3); // trade → TradeUpRewardEvent → the Rare-floored picker opens

    assert_eq!(sim.game_state(), GameState::TalentPicker, "the trade opened the reward picker");
    let offer = sim.pending_offer_ids();
    assert!(!offer.is_empty(), "the trade produced an offer");
    for id in &offer {
        let rarity = sim.talent_rarity(id).unwrap_or_default();
        assert!(
            rarity == "Rare" || rarity == "Epic",
            "offered talent {id} is {rarity}, expected Rare-or-above"
        );
    }
}
