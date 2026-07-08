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
use crate::ability::behavior::{
    BehaviorRegistry, Blink, Bloom, ChannelWhileMoving, ContactMelee, DroppedZone, Grip, HammerCleave,
    LeapToTarget, MeleeCone, NearestMelee, Orbiting, ProjectileBehavior, SelfNova, Summon, TargetedBurst,
};
use crate::ability::hooks::{
    BdkDndDamageBoost, BdkNoHealCapLeechBoost, BloodBoilDndRange, FlamewrathNoConsume, FrostImpaleDeepFreeze,
    FrostImpaleGlacialSpike, HeartStrikeExecuteBonus, HeartStrikeMissingHealth, HookRegistry,
};
use crate::ability::components::{AbilityCooldown, AbilityInstance, CastVfxEvent, Level1Granted, TriggerAbilityEvent, UnlockAbilityEvent};
use crate::ability::systems::bone_shield::bone_shield_on_kill;
use crate::ability::systems::channel::tick_channels;
use crate::ability::systems::execute::{auto_cast_abilities, execute_ready_abilities, tick_ability_cooldowns};
use crate::ability::systems::mage_frost_kill::{frost_charge_on_frostbitten_kill, heal_on_frostbitten_kill};
use crate::ability::systems::purgatory::purgatory_cheat_death;
use crate::ability::systems::summon::{minion_seek_and_face, update_minion_lifecycle};
use crate::core::def_library::DefLibraryAppExt;
use crate::enemy::systems::death::enemy_death;
use crate::core::sets::{CombatSet, MovementSet};
use crate::game::state::GameState;
use crate::hero::assets::{HeroDef, HeroLibrary};
use crate::hero::components::HeroIdentity;
use crate::player::components::Player;
use crate::progression::systems::level_up::handle_level_up;
use crate::player::systems::base_stats::apply_base_stats;
use crate::talent::systems::apply::{install_acquired_talent, uninstall_removed_talent};

pub struct AbilityPlugin;

impl Plugin for AbilityPlugin {
    fn build(&self, app: &mut App) {
        // AbilityDef asset + RON loader + AbilityLibrary + the Startup populate system, in one
        // call (see core/def_library.rs — the generic that replaced the per-type boilerplate).
        app.register_def_library::<AbilityDef>()
            .add_event::<TriggerAbilityEvent>()
            .add_event::<UnlockAbilityEvent>()
            .add_event::<CastVfxEvent>();

        // Built-in behaviors. melee_cone (Phase 1), projectile/self_nova (Phase 3), contact_melee
        // (Phase 5), dropped_zone (Phase 6), blink (Phase 9.1 — the Movement-slot dash), summon /
        // nearest_melee (Phase 9.2 — Companion / Heart Strike), orbiting / hammer_cleave /
        // channel_while_moving (Phase 9.3 — Paladin's Spinning Hammer / Hammer of Justice / Flash
        // of Light), leap_to_target / bloom (Phase 9.4 — Druid's Ferocious Bite+Primal Pounce /
        // Bloom), targeted_burst (Phase 9.5 — Mage's Flamestrike; Flamewrath reuses self_nova
        // as-is). An ability whose behavior is unregistered stays inert.
        let mut behaviors = BehaviorRegistry::default();
        behaviors.register("melee_cone", MeleeCone);
        behaviors.register("projectile", ProjectileBehavior);
        behaviors.register("self_nova", SelfNova);
        behaviors.register("contact_melee", ContactMelee);
        behaviors.register("dropped_zone", DroppedZone);
        behaviors.register("blink", Blink);
        behaviors.register("summon", Summon);
        behaviors.register("nearest_melee", NearestMelee);
        behaviors.register("grip", Grip);
        behaviors.register("orbiting", Orbiting);
        behaviors.register("hammer_cleave", HammerCleave);
        behaviors.register("channel_while_moving", ChannelWhileMoving);
        behaviors.register("leap_to_target", LeapToTarget);
        behaviors.register("bloom", Bloom);
        behaviors.register("targeted_burst", TargetedBurst);
        app.insert_resource(behaviors);

        // Code-driven ability hooks (Phase 6). Talent-gated hooks (in an ability's `hooks` list)
        // run only when the caster has acquired the talent that installs them (ActiveHook);
        // innate hooks (`innate_hooks`) always run if registered — see AbilityDef's doc comment.
        // bone_shield's kill-counting is its own system, not a HookRegistry entry (§8.1(5)/Phase 9.2).
        let mut hooks = HookRegistry::default();
        hooks.register("blood_boil_dnd_range", BloodBoilDndRange);
        hooks.register("heart_strike_missing_health", HeartStrikeMissingHealth);
        hooks.register("heart_strike_execute_bonus", HeartStrikeExecuteBonus);
        hooks.register("bdk_dnd_damage_boost", BdkDndDamageBoost);
        hooks.register("bdk_no_heal_cap", BdkNoHealCapLeechBoost);
        hooks.register("frost_impale_glacial_spike", FrostImpaleGlacialSpike);
        hooks.register("frost_impale_deep_freeze", FrostImpaleDeepFreeze);
        hooks.register("flamewrath_no_consume", FlamewrathNoConsume);
        app.insert_resource(hooks);

        // Ungated by GameState: when several level-ups land in one frame and cross from the
        // AbilityUnlock band into TalentChoices, the UnlockAbilityEvents are written the same
        // frame the state flips to TalentPicker. A reader gated on InRun would skip that frame
        // and the events would expire unread — silently losing the band abilities. Same
        // reasoning as the ungated talent install systems.
        //
        // `.after(CombatSet::Death)` (Phase 9.2 pin): neither system is set-assigned, so absent an
        // explicit anchor its placement relative to the MovementSet/CombatSet chain is decided by
        // the scheduler's internal tie-break — which merely *adding* unrelated systems elsewhere
        // (found via the Companion/summon work) was enough to shift, making a same-frame grant
        // newly visible to that same frame's `execute_ready_abilities` (a whiff-cast burning its
        // cooldown before any target could exist) where before it wasn't. Pinning it to the very
        // end of the per-frame chain — mirroring `gain_experience.after(CombatSet::Death)` below —
        // restores the deterministic "granted this frame ⇒ first fireable next frame" contract the
        // whole ability system (and its tests) already assume, instead of leaving it to chance.
        //
        // `.after(handle_level_up)`: found via Bevy's ambiguity checker while hunting a golden-
        // campaign reproducibility flake. `handle_level_up` writes `UnlockAbilityEvent` (a band
        // ability unlock) that `spawn_unlocked_ability` reads; with no order between them, whether a
        // same-frame band unlock's `AbilityInstance` appears THIS frame or only the NEXT was free to
        // vary between separate schedule builds.
        app.add_systems(
            Update,
            (grant_level_1_abilities, spawn_unlocked_ability)
                .chain()
                .after(CombatSet::Death)
                .after(handle_level_up),
        );
        app.add_systems(
            Update,
            (tick_ability_cooldowns, auto_cast_abilities, execute_ready_abilities, tick_channels)
                .chain()
                .in_set(CombatSet::Damage)
                .run_if(in_state(GameState::InRun)),
        );

        // Purgatory's cheat-death interceptor (Phase 9.2): must see Health AFTER apply_damage has
        // applied a (possibly lethal/negative) hit, and must run before CombatSet::Death's despawn.
        // `.after(apply_heal).after(tick_invulnerability)`: placed at the very end of the
        // core/plugin.rs CombatSet::Apply chain (found via Bevy's ambiguity checker — see that
        // chain's own comment) so it sees the fully-resolved health for the frame, not a value a
        // same-frame heal or invulnerability tick might still race. `.after(install_acquired_talent)
        // .after(uninstall_removed_talent).after(apply_base_stats)`: same reasoning as
        // talent/plugin.rs's class-passive consumers — resolve_params reads AcquiredTalents/
        // Health.max, both of which these can mutate the same frame.
        app.add_systems(
            Update,
            purgatory_cheat_death
                .in_set(CombatSet::Apply)
                .after(crate::core::systems::apply_damage::apply_damage)
                .after(crate::core::systems::apply_heal::apply_heal)
                .after(crate::core::systems::invulnerability::tick_invulnerability)
                .after(install_acquired_talent)
                .after(uninstall_removed_talent)
                .after(apply_base_stats)
                .run_if(in_state(GameState::InRun)),
        );

        // Bone Shield's kill counter (Phase 9.2, Death Strike's epic talent): reads Health/
        // LastHitBy on dying enemies before enemy_death despawns them. The Mage's two frost-kill
        // passives (Phase 9.5) are the identical shape, one more read (frostbite status).
        //
        // `.before(enemy_death)` (Phase 9.5 pin — found while adding the two Mage systems below):
        // NOT actually order-agnostic, despite this comment's own prior claim — `enemy_death`'s
        // `commands.despawn()` is a Bevy auto-inserted sync point (Commands-issuing systems get an
        // automatic `apply_deferred` immediately after them), so if `enemy_death` merely happens to
        // execute FIRST in the scheduler's tie-break order for this unordered set, the entity is
        // already gone by the time a same-set reader runs — no despawn-visibility grace period
        // exists within CombatSet::Death itself. Adding the two new systems below shifted that
        // tie-break order enough to make `talent::systems::passives::overkill_leech_on_kill`
        // (a DIFFERENT, already-shipped Death-set reader) start losing this race — caught by
        // `tests/bdk_class_passives.rs`, not a new bug in the code below, but a latent one this
        // change happened to expose (the same class of gap every sub-phase's first real stress of
        // an assumption has surfaced). Fixed at the root: every Death-set system that reads a dying
        // `Enemy`'s `Health`/`LastHitBy` now explicitly runs `.before(enemy_death)` — see also
        // `talent/plugin.rs`'s identical pin on `overkill_leech_on_kill`.
        app.add_systems(
            Update,
            (bone_shield_on_kill, frost_charge_on_frostbitten_kill, heal_on_frostbitten_kill)
                .before(enemy_death)
                .in_set(CombatSet::Death)
                .run_if(in_state(GameState::InRun)),
        );

        // Minion lifecycle (Phase 9.2 — Companion). Seeking/facing is AI-shaped, so it belongs in
        // MovementSet::Intent alongside the enemy flow-field follower / ranged-caster AI; the
        // lifecycle reaper runs after death resolves so a killing hit this frame is seen.
        //
        // `.after(spawn_unlocked_ability)` (found via the golden-master reproducibility flake this
        // pins): both this system and `spawn_unlocked_ability` are merely anchored
        // `.after(CombatSet::Death)`, with NO order between the two of them — and both issue
        // entity-spawning/despawning Commands (a granted ability spawns an AbilityInstance; an
        // expired minion despawns itself + its owned instance). Whichever ran first determined
        // which Commands got queued (and thus which entity indices got allocated) first — an
        // unpinned ambiguity Bevy can resolve differently between separate schedule builds, so two
        // runs of the identical seeded script could allocate DIFFERENT entity indices for the SAME
        // semantic set of alive entities. That divergence is invisible until it flips a
        // nearest-neighbor tie-break somewhere downstream (here: the golden-campaign bot's "nearest
        // enemy" scan), at which point it becomes a real, observable trace difference. Exactly the
        // same underlying risk class as the Companion grant/execute race fixed earlier this phase —
        // an unordered pair of Command-issuing systems sharing an anchor point, not a set.
        app.add_systems(
            Update,
            (
                minion_seek_and_face.in_set(MovementSet::Intent),
                update_minion_lifecycle.after(CombatSet::Death).after(spawn_unlocked_ability),
            )
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
