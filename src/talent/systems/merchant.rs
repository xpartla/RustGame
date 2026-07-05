// Merchant operations (Phase 7.5E, D2) — talent removal and the 3-for-1 trade-up.
//
// The consumer side has existed since Phase 2: `uninstall_removed_talent` handles `TalentRemovedEvent`
// (updates AcquiredTalents / pops ActiveHooks). These handlers drive it, and the trade-up reuses the
// level-up TalentPicker flow with a rarity floor (the same machinery the ThroneRoom kiss uses).
//
// Flow: the merchant overlay's input (`handle_merchant_input`, GameState::Merchant) emits a
// MerchantRemoveRequest / MerchantTradeRequest; the ops handlers apply them. A remove takes effect in
// place (the player stays in the shop); a trade removes the three sacrifices and emits a
// TradeUpRewardEvent, which progression turns into a Rare-or-better picker.
//
// Golden-master note: the campaign never enters a Merchant node, and every handler is gated on its
// state / request event, so this module is inert in the campaign ⇒ byte-identical.

use bevy::prelude::*;
use crate::game::state::GameState;
use crate::player::components::Player;
use crate::talent::assets::{TalentDef, TalentId, TalentLibrary, TalentRarity};
use crate::talent::components::AcquiredTalents;
use crate::talent::systems::apply::TalentRemovedEvent;

/// Remove the named talent from the player (emitted by the merchant overlay).
#[derive(Event, Debug)]
pub struct MerchantRemoveRequest {
    pub owner: Entity,
    pub talent_id: TalentId,
}

/// Sacrifice three talents for one higher-rarity pick (emitted by the merchant overlay).
#[derive(Event, Debug)]
pub struct MerchantTradeRequest {
    pub owner: Entity,
    pub sacrifice: [TalentId; 3],
}

/// A completed trade owes the player a rarity-floored pick; progression (offer.rs) opens the picker.
#[derive(Event, Debug)]
pub struct TradeUpRewardEvent {
    pub owner: Entity,
    pub min_rarity: TalentRarity,
}

/// Applies each remove request by emitting `TalentRemovedEvent` (consumed by `uninstall_removed_talent`).
pub fn handle_merchant_remove(
    mut requests: EventReader<MerchantRemoveRequest>,
    mut removed: EventWriter<TalentRemovedEvent>,
) {
    for req in requests.read() {
        removed.write(TalentRemovedEvent { owner: req.owner, talent_id: req.talent_id.clone() });
    }
}

/// Applies each trade request: removes the three sacrifices and owes a pick one rarity above the
/// highest sacrificed (Common⇒Rare, Rare⇒Epic, capped at Epic).
pub fn handle_merchant_trade(
    mut requests: EventReader<MerchantTradeRequest>,
    mut removed: EventWriter<TalentRemovedEvent>,
    mut rewards: EventWriter<TradeUpRewardEvent>,
    library: Res<TalentLibrary>,
    defs: Res<Assets<TalentDef>>,
) {
    for req in requests.read() {
        let mut highest = TalentRarity::Common;
        for id in &req.sacrifice {
            removed.write(TalentRemovedEvent { owner: req.owner, talent_id: id.clone() });
            if let Some(rarity) = library.get(id).and_then(|h| defs.get(h)).map(|d| d.rarity.clone()) {
                if rarity > highest {
                    highest = rarity;
                }
            }
        }
        rewards.write(TradeUpRewardEvent { owner: req.owner, min_rarity: next_rarity_above(&highest) });
    }
}

/// The rarity one step above `r`, capped at Epic (the highest tier).
fn next_rarity_above(r: &TalentRarity) -> TalentRarity {
    match r {
        TalentRarity::Common => TalentRarity::Rare,
        TalentRarity::Rare => TalentRarity::Epic,
        TalentRarity::Epic => TalentRarity::Epic,
    }
}

/// Merchant overlay input (GameState::Merchant): a digit removes the acquired talent at that index; T
/// trades the first three; Esc leaves to the map. Keyboard-first (D4); the overlay lists the talents
/// with matching numbers.
pub fn handle_merchant_input(
    keys: Res<ButtonInput<KeyCode>>,
    players: Query<(Entity, &AcquiredTalents), With<Player>>,
    mut remove: EventWriter<MerchantRemoveRequest>,
    mut trade: EventWriter<MerchantTradeRequest>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::MapSelect); // leave the shop → choose the next node
        return;
    }
    let Ok((owner, acquired)) = players.single() else {
        return;
    };
    if keys.just_pressed(KeyCode::KeyT) {
        if acquired.entries.len() >= 3 {
            let sacrifice = [
                acquired.entries[0].0.clone(),
                acquired.entries[1].0.clone(),
                acquired.entries[2].0.clone(),
            ];
            trade.write(MerchantTradeRequest { owner, sacrifice });
        }
        return;
    }
    if let Some(i) = pressed_digit(&keys) {
        if let Some((id, _)) = acquired.entries.get(i) {
            remove.write(MerchantRemoveRequest { owner, talent_id: id.clone() });
        }
    }
}

/// Maps a just-pressed digit (1–9) to a 0-based index.
fn pressed_digit(keys: &ButtonInput<KeyCode>) -> Option<usize> {
    const DIGITS: [KeyCode; 9] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    DIGITS.iter().position(|k| keys.just_pressed(*k))
}
