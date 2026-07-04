// AbilityPlugin — wires the ability system into the app.
//
// Responsibilities:
//   - Registers AbilityDef as a Bevy asset + its RON loader.
//   - Registers BehaviorRegistry (with the implemented built-in behaviors) and HookRegistry.
//   - Registers TriggerAbilityEvent.
//   - Loads the ability RON files into AbilityLibrary at startup.
//   - Grants the starting ability (Phase 1 stub) and runs cooldown/execution each frame.
//
// All runtime systems run in InState(GameState::InRun) and in CombatSet::Damage.

use bevy::asset::AssetApp;
use bevy::prelude::*;
use crate::ability::assets::{AbilityDef, AbilityDefLoader, AbilityLibrary};
use crate::ability::behavior::{BehaviorRegistry, MeleeCone};
use crate::ability::components::{AbilityCooldown, AbilityInstance, TriggerAbilityEvent};
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
            .add_event::<TriggerAbilityEvent>();

        // Built-in behaviors. Phase 1 implements only melee_cone; others register in their phase.
        let mut behaviors = BehaviorRegistry::default();
        behaviors.register("melee_cone", MeleeCone);
        app.insert_resource(behaviors);

        app.add_systems(Startup, load_ability_defs);
        app.add_systems(
            Update,
            grant_starting_abilities.run_if(in_state(GameState::InRun)),
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

/// PHASE 1 STUB: grants the Blood Death Knight's level-1 ability to a freshly spawned player.
/// Phase 2 replaces this with progression-driven UnlockAbilityEvents; Phase 4 sources the
/// starting list from HeroDef.level_1_abilities.
fn grant_starting_abilities(mut commands: Commands, players: Query<Entity, Added<Player>>) {
    for owner in &players {
        commands.spawn((
            AbilityInstance { def_id: "death_strike".to_string(), owner },
            AbilityCooldown::new(0.0),
        ));
    }
}
