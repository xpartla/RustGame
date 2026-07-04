// Maintains AcquiredTalents and ActiveHooks on the player entity.
//
// Listen for:
//   TalentAcquiredEvent  — adds the talent to AcquiredTalents; if its effect is Behavior(id),
//                          pushes id into ActiveHooks.
//   TalentRemovedEvent   — removes one copy from AcquiredTalents; if the effect is Behavior(id)
//                          and the count drops to 0, pops id from ActiveHooks.
//
// These events are emitted by:
//   - progression/systems/offer.rs on player choice (TalentAcquiredEvent)
//   - talent/systems/merchant.rs on remove-talent and trade-up (TalentRemovedEvent) — Phase 8
//
// Modifier talents (TalentEffect::Modifier) need no component installation — they are evaluated
// on the fly by talent/modifier.rs::resolve_params at ability-fire time. Behavior *execution*
// (running AbilityDef.hooks during a cast) is deferred until the first real hook lands; Phase 2
// only maintains the ActiveHooks data so that path is ready.
//
// These systems run ungated by GameState: a TalentAcquiredEvent is emitted from the
// TalentPicker state, so the reader must not be frozen with the InRun world.

use bevy::prelude::*;
use crate::talent::assets::{TalentDef, TalentEffect, TalentId, TalentLibrary};
use crate::talent::components::{AcquiredTalents, ActiveHooks};
use crate::player::components::Player;

#[derive(Event, Debug)]
pub struct TalentAcquiredEvent {
    pub owner: Entity,
    pub talent_id: TalentId,
}

#[derive(Event, Debug)]
pub struct TalentRemovedEvent {
    pub owner: Entity,
    pub talent_id: TalentId,
}

/// Inserts the talent bookkeeping components on a freshly spawned player. Keeps the `player`
/// module decoupled from `talent` (mirrors how the ability plugin grants starting abilities).
pub fn attach_talent_components(mut commands: Commands, players: Query<Entity, Added<Player>>) {
    for entity in &players {
        commands
            .entity(entity)
            .insert((AcquiredTalents::default(), ActiveHooks::default()));
    }
}

/// Applies each TalentAcquiredEvent to the owner's talent state.
pub fn install_acquired_talent(
    mut events: EventReader<TalentAcquiredEvent>,
    mut players: Query<(&mut AcquiredTalents, &mut ActiveHooks)>,
    talent_defs: Res<Assets<TalentDef>>,
    library: Res<TalentLibrary>,
) {
    for ev in events.read() {
        let Ok((mut acquired, mut hooks)) = players.get_mut(ev.owner) else {
            continue;
        };
        acquired.add(ev.talent_id.clone());

        if let Some(def) = library.get(&ev.talent_id).and_then(|h| talent_defs.get(h)) {
            if let TalentEffect::Behavior(hook_id) = &def.effect {
                hooks.add(hook_id.clone());
            }
        }
    }
}

/// Applies each TalentRemovedEvent (merchant remove / trade-up, Phase 8).
pub fn uninstall_removed_talent(
    mut events: EventReader<TalentRemovedEvent>,
    mut players: Query<(&mut AcquiredTalents, &mut ActiveHooks)>,
    talent_defs: Res<Assets<TalentDef>>,
    library: Res<TalentLibrary>,
) {
    for ev in events.read() {
        let Ok((mut acquired, mut hooks)) = players.get_mut(ev.owner) else {
            continue;
        };
        acquired.remove_one(&ev.talent_id);

        // Only drop the hook once the last copy is gone.
        if acquired.count_of(&ev.talent_id) == 0 {
            if let Some(def) = library.get(&ev.talent_id).and_then(|h| talent_defs.get(h)) {
                if let TalentEffect::Behavior(hook_id) = &def.effect {
                    hooks.remove(hook_id);
                }
            }
        }
    }
}
