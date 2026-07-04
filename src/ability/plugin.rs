// AbilityPlugin — wires the ability system into the app.
//
// Responsibilities:
//   - Registers AbilityDef as a Bevy asset + its RON loader.
//   - Registers BehaviorRegistry (with the implemented built-in behaviors). The talent
//     hook execution path returns with a real hook in a later phase.
//   - Registers TriggerAbilityEvent and UnlockAbilityEvent.
//   - Loads the ability RON files into AbilityLibrary at startup.
//   - Grants the level-1 abilities (Phase-2 stub) via UnlockAbilityEvent, spawns an
//     AbilityInstance per unlock, and runs cooldown/execution each frame.
//
// All runtime systems run in InState(GameState::InRun). Execution runs in CombatSet::Damage.

use bevy::asset::AssetApp;
use bevy::prelude::*;
use crate::ability::assets::{AbilityDef, AbilityDefLoader, AbilityId, AbilityLibrary};
use crate::ability::behavior::{BehaviorRegistry, MeleeCone};
use crate::ability::components::{AbilityCooldown, AbilityInstance, TriggerAbilityEvent, UnlockAbilityEvent};
use crate::ability::systems::execute::{execute_ready_abilities, tick_ability_cooldowns};
use crate::core::sets::CombatSet;
use crate::game::state::GameState;
use crate::player::components::Player;

pub struct AbilityPlugin;

impl Plugin for AbilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<AbilityDef>()
            .register_asset_loader(AbilityDefLoader)
            .init_resource::<AbilityLibrary>()
            .add_event::<TriggerAbilityEvent>()
            .add_event::<UnlockAbilityEvent>();

        // Built-in behaviors. Phase 1 implements only melee_cone; others register in their phase.
        let mut behaviors = BehaviorRegistry::default();
        behaviors.register("melee_cone", MeleeCone);
        app.insert_resource(behaviors);

        app.add_systems(Startup, load_ability_defs);
        app.add_systems(
            Update,
            (grant_level_1_abilities, spawn_unlocked_ability)
                .chain()
                .run_if(in_state(GameState::InRun)),
        );
        app.add_systems(
            Update,
            (tick_ability_cooldowns, execute_ready_abilities)
                .chain()
                .in_set(CombatSet::Damage)
                .run_if(in_state(GameState::InRun)),
        );
    }
}

/// Loads each ability RON into the AbilityLibrary, keyed by its id.
/// Phase 1 loads a fixed list; a later phase can scan the `abilities/` folder.
fn load_ability_defs(asset_server: Res<AssetServer>, mut library: ResMut<AbilityLibrary>) {
    const ABILITIES: &[(&str, &str)] = &[
        ("death_strike", "abilities/death_strike.ability.ron"),
        ("dnd", "abilities/dnd.ability.ron"),
    ];
    for (id, path) in ABILITIES {
        library.defs.insert((*id).to_string(), asset_server.load(*path));
    }
}

/// PHASE 2 STUB: grants the Blood Death Knight's level-1 abilities to a freshly spawned player
/// by emitting an UnlockAbilityEvent for each. Phase 4 sources this list from
/// HeroDef.level_1_abilities instead of hardcoding it. The band abilities unlocked at L2–L6 flow
/// through the same UnlockAbilityEvent path from progression/systems/level_up.rs.
///
/// Only `death_strike` has a registered behavior in Phase 2; `dnd`/`companion` become inert
/// AbilityInstances (no behavior, no input binding, no auto-cast yet) until their phases land.
fn grant_level_1_abilities(
    mut unlocks: EventWriter<UnlockAbilityEvent>,
    players: Query<Entity, Added<Player>>,
) {
    const LEVEL_1: &[&str] = &["death_strike", "dnd", "companion"];
    for owner in &players {
        for id in LEVEL_1 {
            unlocks.write(UnlockAbilityEvent { ability_id: (*id).to_string(), owner });
        }
    }
}

/// Spawns one AbilityInstance entity per UnlockAbilityEvent. Idempotent: an already-owned
/// ability id is skipped so a duplicate unlock never stacks a second instance.
fn spawn_unlocked_ability(
    mut commands: Commands,
    mut unlocks: EventReader<UnlockAbilityEvent>,
    existing: Query<&AbilityInstance>,
) {
    // Snapshot of what each owner already has, plus what we grant this frame (so two events in
    // one frame for the same id don't double-spawn).
    let mut owned: Vec<(Entity, AbilityId)> = existing
        .iter()
        .map(|i| (i.owner, i.def_id.clone()))
        .collect();

    for ev in unlocks.read() {
        let already = owned.iter().any(|(o, id)| *o == ev.owner && *id == ev.ability_id);
        if already {
            continue;
        }
        commands.spawn((
            AbilityInstance { def_id: ev.ability_id.clone(), owner: ev.owner },
            // Start ready; execute re-reads the resolved "cooldown" param on each cast.
            AbilityCooldown::new(0.0),
        ));
        owned.push((ev.owner, ev.ability_id.clone()));
    }
}
