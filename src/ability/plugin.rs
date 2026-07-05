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

use bevy::prelude::*;
use crate::ability::assets::{AbilityDef, AbilityId};
use crate::ability::behavior::{BehaviorRegistry, ContactMelee, MeleeCone, ProjectileBehavior, SelfNova};
use crate::ability::components::{AbilityCooldown, AbilityInstance, Level1Granted, TriggerAbilityEvent, UnlockAbilityEvent};
use crate::ability::systems::execute::{auto_cast_abilities, execute_ready_abilities, tick_ability_cooldowns};
use crate::core::def_library::DefLibraryAppExt;
use crate::core::sets::CombatSet;
use crate::game::state::GameState;
use crate::hero::assets::{HeroDef, HeroLibrary};
use crate::hero::components::HeroIdentity;
use crate::player::components::Player;

pub struct AbilityPlugin;

impl Plugin for AbilityPlugin {
    fn build(&self, app: &mut App) {
        // AbilityDef asset + RON loader + AbilityLibrary + the Startup populate system, in one
        // call (see core/def_library.rs — the generic that replaced the per-type boilerplate).
        app.register_def_library::<AbilityDef>()
            .add_event::<TriggerAbilityEvent>()
            .add_event::<UnlockAbilityEvent>();

        // Built-in behaviors. melee_cone (Phase 1), projectile (Phase 3). Zone/orbit/summon/…
        // register in their own phases; an ability whose behavior is unregistered stays inert.
        let mut behaviors = BehaviorRegistry::default();
        behaviors.register("melee_cone", MeleeCone);
        behaviors.register("projectile", ProjectileBehavior);
        behaviors.register("self_nova", SelfNova);
        behaviors.register("contact_melee", ContactMelee);
        app.insert_resource(behaviors);

        // Ungated by GameState: when several level-ups land in one frame and cross from the
        // AbilityUnlock band into TalentChoices, the UnlockAbilityEvents are written the same
        // frame the state flips to TalentPicker. A reader gated on InRun would skip that frame
        // and the events would expire unread — silently losing the band abilities. Same
        // reasoning as the ungated talent install systems.
        app.add_systems(
            Update,
            (grant_level_1_abilities, spawn_unlocked_ability).chain(),
        );
        app.add_systems(
            Update,
            (tick_ability_cooldowns, auto_cast_abilities, execute_ready_abilities)
                .chain()
                .in_set(CombatSet::Damage)
                .run_if(in_state(GameState::InRun)),
        );
    }
}

/// Grants a hero's level-1 abilities by emitting an UnlockAbilityEvent for each id in
/// `HeroDef.level_1_abilities` (Phase 4 — was a hardcoded stub). Deferred rather than run on
/// `Added<Player>` because the HeroDef asset loads asynchronously; the `Level1Granted` marker
/// makes it fire exactly once per player, the frame its HeroDef becomes available. Band abilities
/// (L2–L6) flow through the same UnlockAbilityEvent path from progression/systems/level_up.rs.
///
/// Abilities whose behavior isn't registered yet (e.g. `dnd`/`companion`) become inert
/// AbilityInstances (no behavior, no input binding, no auto-cast) until their phases land.
fn grant_level_1_abilities(
    mut commands: Commands,
    mut unlocks: EventWriter<UnlockAbilityEvent>,
    hero_library: Res<HeroLibrary>,
    hero_defs: Res<Assets<HeroDef>>,
    players: Query<(Entity, &HeroIdentity), (With<Player>, Without<Level1Granted>)>,
) {
    for (owner, hero_id) in &players {
        let Some(handle) = hero_library.get(&hero_id.0) else { continue };
        let Some(hero_def) = hero_defs.get(handle) else { continue };
        for id in &hero_def.level_1_abilities {
            unlocks.write(UnlockAbilityEvent { ability_id: id.clone(), owner });
        }
        commands.entity(owner).insert(Level1Granted);
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
