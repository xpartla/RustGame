// Talent-offer refill and player-choice handling for the TalentPicker overlay.
//
// Two systems run in GameState::TalentPicker (refill_offer ordered before handle_talent_choice):
//   refill_offer        — closes the overlay when the backlog is drained; otherwise lazily
//                         generates the next offer into LevelUpFlowState.pending_offer. Lazy
//                         generation means each offer sees the talents acquired so far, so
//                         uniqueness constraints stay correct across a multi-level backlog.
//   handle_talent_choice — reads 1/2/3 to pick an option (emits TalentAcquiredEvent) or Esc to
//                         decline; either way consumes one owed choice.
//
// The UI (ui/screens/talent_picker.rs) only reads pending_offer to render — it owns no state and
// handles no input. install_acquired_talent (talent module, ungated) applies the emitted event.
//
// The eligible pool is the union of the player's unlocked abilities' talent_pools, the class
// passive pool, and general passives. Ids with no loaded TalentDef self-filter in generate_offer,
// so unimplemented content (band abilities, class passives without RON files) contributes nothing.

use bevy::prelude::*;
use crate::ability::assets::{AbilityDef, AbilityLibrary};
use crate::ability::components::AbilityInstance;
use crate::game::state::GameState;
use crate::player::components::Player;
use crate::progression::state::LevelUpFlowState;
use crate::run::rng::RunRng;
use crate::talent::assets::{TalentDef, TalentId, TalentLibrary};
use crate::talent::components::AcquiredTalents;
use crate::talent::offer::{generate_offer, OfferContext};
use crate::talent::systems::apply::TalentAcquiredEvent;

// PHASE 2 STUB: hardcoded Blood Death Knight class passive pool. These have no TalentDef RON
// files yet, so they self-filter out of offers; listed here so the pool is faithful once the
// content pass adds them. Phase 4 sources this from HeroDef.class_passive_pool.
const BDK_CLASS_PASSIVES: &[&str] = &[
    "bdk_passive_no_heal_cap",
    "bdk_passive_dnd_damage_boost",
    "bdk_passive_overkill_leech",
    "bdk_passive_health_and_healing",
    "bdk_passive_blood_boil_spawns_dnd",
];

// Emitted by the ThroneRoom encounter (Phase 7, run/systems/transitions.rs::load_encounter).
#[derive(Event, Debug)]
pub struct ThroneRoomRewardEvent {
    pub owner: Entity,
}

/// Ensures the overlay always has an offer to show while choices are owed, and closes it when
/// the backlog is drained. Ordered before `handle_talent_choice`.
pub fn refill_offer(
    mut flow: ResMut<LevelUpFlowState>,
    mut next_state: ResMut<NextState<GameState>>,
    mut rng: ResMut<RunRng>,
    players: Query<(Entity, &AcquiredTalents), With<Player>>,
    instances: Query<&AbilityInstance>,
    ability_library: Res<AbilityLibrary>,
    ability_defs: Res<Assets<AbilityDef>>,
    talent_defs: Res<Assets<TalentDef>>,
    talent_library: Res<TalentLibrary>,
) {
    if flow.owed_choices == 0 {
        flow.pending_offer = None;
        next_state.set(GameState::InRun);
        return;
    }
    if flow.pending_offer.is_some() {
        return; // already showing an offer
    }
    let Ok((player, acquired)) = players.single() else {
        return;
    };
    let eligible = build_eligible_pool(player, &instances, &ability_library, &ability_defs);
    let offer = generate_offer(
        OfferContext::LevelUp,
        &eligible,
        acquired,
        &talent_defs,
        &talent_library,
        &mut rng,
    );
    flow.pending_offer = Some(offer);
}

/// Reads the player's pick (1/2/3) or decline (Esc) and consumes one owed choice.
pub fn handle_talent_choice(
    keys: Res<ButtonInput<KeyCode>>,
    mut flow: ResMut<LevelUpFlowState>,
    mut acquired_events: EventWriter<TalentAcquiredEvent>,
    players: Query<Entity, With<Player>>,
) {
    // Only act while an offer is on screen.
    let Some(offer) = flow.pending_offer.clone() else {
        return;
    };

    let selected: Option<usize> = if keys.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else {
        None
    };
    let declined = keys.just_pressed(KeyCode::Escape);

    if selected.is_none() && !declined {
        return; // no relevant input this frame
    }

    if let Some(idx) = selected {
        match offer.options.get(idx) {
            Some(talent_id) => {
                if let Ok(player) = players.single() {
                    acquired_events.write(TalentAcquiredEvent {
                        owner: player,
                        talent_id: talent_id.clone(),
                    });
                }
            }
            None => return, // pressed a number past the offered options — ignore
        }
    }

    // A valid pick or a decline consumes one owed choice; refill_offer closes the overlay or
    // generates the next offer on the following frame.
    flow.pending_offer = None;
    flow.owed_choices = flow.owed_choices.saturating_sub(1);
}

/// Handles the ThroneRoom kiss (Phase 7F): on entering a ThroneRoom, offer 1 of 3 **Rare-or-better**
/// talents before the fight. Reuses the level-up TalentPicker flow — generate a `ThroneRoom`-context
/// offer (Rare rarity floor), owe one choice, and enter the picker (which freezes the InRun world). The
/// backlog drains through the same `refill_offer` / `handle_talent_choice` pair.
pub fn handle_throne_room_reward(
    mut events: EventReader<ThroneRoomRewardEvent>,
    mut flow: ResMut<LevelUpFlowState>,
    mut next_state: ResMut<NextState<GameState>>,
    mut rng: ResMut<RunRng>,
    players: Query<(Entity, &AcquiredTalents), With<Player>>,
    instances: Query<&AbilityInstance>,
    ability_library: Res<AbilityLibrary>,
    ability_defs: Res<Assets<AbilityDef>>,
    talent_defs: Res<Assets<TalentDef>>,
    talent_library: Res<TalentLibrary>,
) {
    if events.is_empty() {
        return;
    }
    events.clear();
    let Ok((player, acquired)) = players.single() else {
        return;
    };
    let eligible = build_eligible_pool(player, &instances, &ability_library, &ability_defs);
    let offer = generate_offer(
        OfferContext::ThroneRoom,
        &eligible,
        acquired,
        &talent_defs,
        &talent_library,
        &mut rng,
    );
    flow.pending_offer = Some(offer);
    flow.owed_choices += 1;
    next_state.set(GameState::TalentPicker);
}

/// Builds the union of talent ids the player could currently be offered.
fn build_eligible_pool(
    player: Entity,
    instances: &Query<&AbilityInstance>,
    ability_library: &AbilityLibrary,
    ability_defs: &Assets<AbilityDef>,
) -> Vec<TalentId> {
    let mut pool: Vec<TalentId> = Vec::new();

    // Talents from each unlocked ability the player owns.
    for inst in instances.iter() {
        if inst.owner != player {
            continue;
        }
        if let Some(def) = ability_library.get(&inst.def_id).and_then(|h| ability_defs.get(h)) {
            for t in &def.talent_pool {
                push_unique(&mut pool, t);
            }
        }
    }

    // Class passives (+ general passives, none yet).
    for t in BDK_CLASS_PASSIVES {
        push_unique(&mut pool, t);
    }

    pool
}

fn push_unique(pool: &mut Vec<TalentId>, id: &str) {
    if !pool.iter().any(|p| p == id) {
        pool.push(id.to_string());
    }
}
