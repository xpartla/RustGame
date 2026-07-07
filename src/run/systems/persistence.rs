// Save cadence + resume hydration (Phase 8, §3 of docs/phase8-plan.md).
//
// Two halves:
//   - The save side: `sync_run_state` mirrors the live player's abilities/talents/level-flow into
//     `RunState` (it is otherwise never written after run-start — see run/state.rs's header),
//     `tick_run_timer` accumulates the deterministic run clock (D2), and `save_run_snapshot` /
//     `record_run_end` are the two save-point actions `handle_encounter_complete` /
//     `player_death` call at every node boundary / run end (§3.1). Both write into
//     `MetaState.in_progress_run` / `run_history` only — the windowed autosave
//     (meta/persistence.rs, gated on `resource_changed::<MetaState>`) is what actually touches
//     disk, so this whole module is headless-safe and sim-able.
//   - The resume side: `resume_run` (§3.2) is the mirror of `reset.rs::reset_and_start_run` — it
//     reuses `teardown_run`/`respawn_player` and the idempotent unlock/talent-install paths so a
//     resumed run needs no new spawn code. Because `RunRng` is restored to its exact saved stream
//     position (the D1 payoff), the roster the resumed room rolls is identical to an uninterrupted
//     run — the headline scenario test (tests/persistence.rs).

use bevy::prelude::*;

use crate::ability::components::{AbilityInstance, UnlockAbilityEvent};
use crate::core::components::Health;
use crate::game::state::GameState;
use crate::meta::score::{compute_score, ScoreInput};
use crate::meta::state::{unlock_heroes_on_progress, MetaState, RunRecord, SavedRun};
use crate::player::components::{Experience, Player};
use crate::progression::state::LevelUpFlowState;
use crate::run::rng::RunRng;
use crate::run::state::{node_depth, CurrentEncounter, RunState};
use crate::run::systems::reset::{respawn_player, teardown_run};
use crate::talent::components::{AcquiredTalents, ActiveHooks, BoneShieldProgress};
use crate::talent::systems::apply::TalentAcquiredEvent;

/// Ticks the deterministic run clock (D2) while a run is live. `Time::delta` only advances on
/// frames the `Update` schedule actually runs (frozen behind any overlay, since every overlay
/// state pauses `Update`'s gameplay-adjacent systems the same way), so pausing doesn't inflate it.
/// Absent `RunState` (the golden campaign never starts a run) ⇒ inert by construction.
pub fn tick_run_timer(time: Res<Time>, mut run_state: ResMut<RunState>) {
    run_state.elapsed_secs += time.delta_secs();
}

/// Mirrors the live player's abilities/talents/level-flow/vitals into `run_state` — the "make the
/// resumable snapshot reflect reality" step (§1.2). Without this, `unlocked_abilities` /
/// `acquired_talents` would serialize empty (they are otherwise never written after run-start).
pub fn sync_run_state(
    run_state: &mut RunState,
    health: f32,
    level: u32,
    owner: Entity,
    owned_abilities: &Query<(Entity, &AbilityInstance)>,
    acquired: &AcquiredTalents,
    level_flow: &LevelUpFlowState,
) {
    run_state.player_health = health;
    run_state.player_level = level;
    run_state.unlocked_abilities = owned_abilities
        .iter()
        .filter(|(_, i)| i.owner == owner)
        .map(|(_, i)| i.def_id.clone())
        .collect();
    run_state.acquired_talents = acquired.entries.clone();
    run_state.level_flow = level_flow.clone();
}

/// Snapshots the (already-synced) `run` + the live `rng` into `MetaState.in_progress_run` — the
/// save action for every non-terminal node boundary (§3.1: `EncounterCompleteEvent` / entering
/// `MapSelect` / a non-final act advance). The windowed autosave system persists this to disk
/// whenever `MetaState` changes; the sim only ever touches this in-memory field.
pub fn save_run_snapshot(meta: &mut MetaState, run: &RunState, rng: &RunRng) {
    meta.in_progress_run = Some(SavedRun { run: run.clone(), rng: rng.clone() });
}

/// The run-end action (§3.1): defeat (`player_death`) or the Act-3 boss clear
/// (`handle_encounter_complete`). Computes the score (D2), appends a `RunRecord`, clears the saved
/// in-progress run (there is nothing to resume once a run has ended), and calls the Phase-9
/// hero-unlock seam (inert today, D3).
pub fn record_run_end(meta: &mut MetaState, run: &RunState, victory: bool) {
    let node_column = run.act_graph.node(run.current_node).map(|n| n.column).unwrap_or(0);
    let score = compute_score(&ScoreInput {
        act: run.current_act,
        node_column,
        level: run.player_level,
        victory,
        elapsed_secs: run.elapsed_secs,
    });
    let timestamp_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    meta.run_history.push(RunRecord {
        hero_id: run.hero_id.clone(),
        act_reached: run.current_act,
        score,
        timestamp_unix,
    });
    meta.in_progress_run = None;
    unlock_heroes_on_progress(meta, run, victory);
}

// ── Resume ───────────────────────────────────────────────────────────────────────────────────

/// Emitted by the main menu's "Resume Run" input (only enabled when `MetaState.in_progress_run`
/// is `Some`); consumed by `apply_resume_request`.
#[derive(Event, Debug, Clone, Default)]
pub struct ResumeRunRequest;

/// Re-hydrates a saved run into a live one (§3.2) — the mirror of `reset_and_start_run`, reusing
/// its teardown/respawn primitives so resume needs no new spawn code. Because `saved.rng` restores
/// the exact stream position (D1), the room `load_encounter` rolls next frame for `current_node` is
/// byte-identical to what an uninterrupted run would have rolled at that same point.
pub fn resume_run(world: &mut World, saved: SavedRun) {
    teardown_run(world);

    world.insert_resource(saved.rng);
    respawn_player(world, &saved.run.hero_id);
    world.flush();

    let mut players = world.query_filtered::<Entity, With<Player>>();
    let Some(player) = players.iter(world).next() else {
        return; // spawn_player always succeeds; defensive only
    };

    if let Some(mut health) = world.get_mut::<Health>(player) {
        health.current = saved.run.player_health;
    }
    if let Some(mut exp) = world.get_mut::<Experience>(player) {
        exp.level = saved.run.player_level;
        exp.current = 0;
        exp.to_next = Experience::to_next_for(saved.run.player_level);
    }

    // Attach the talent bookkeeping components synchronously (rather than waiting on
    // `attach_talent_components`'s own unordered Update turn): the TalentAcquiredEvent replay
    // below targets this same just-spawned player *this frame*, and `install_acquired_talent`
    // needs `AcquiredTalents`/`ActiveHooks` to already exist or it silently drops the event.
    // `attach_talent_components` is guarded `Without<AcquiredTalents>` so it won't clobber this.
    // BoneShieldProgress (Phase 9.2) resets on resume too — DP3 (phase9-plan.md): mid-encounter/
    // talent-progress-style state is transient, matching Charges' own resume behavior.
    world.entity_mut(player).insert((AcquiredTalents::default(), ActiveHooks::default(), BoneShieldProgress::default()));

    // Re-grant abilities (incl. level-1) through the idempotent UnlockAbilityEvent →
    // spawn_unlocked_ability path — no new spawn code (§3.2 step 4).
    for ability_id in &saved.run.unlocked_abilities {
        world.send_event(UnlockAbilityEvent { ability_id: ability_id.clone(), owner: player });
    }
    // Re-install talents (one TalentAcquiredEvent per copy) through the existing
    // install_acquired_talent path — rebuilds AcquiredTalents + ActiveHooks (§3.2 step 5).
    for (talent_id, count) in &saved.run.acquired_talents {
        for _ in 0..*count {
            world.send_event(TalentAcquiredEvent { owner: player, talent_id: talent_id.clone() });
        }
    }

    world.insert_resource(saved.run.level_flow.clone());

    let node = saved
        .run
        .act_graph
        .node(saved.run.current_node)
        .expect("saved current_node exists in the saved act_graph")
        .clone();
    let depth = node_depth(saved.run.current_act, node.column);
    world.insert_resource(CurrentEncounter::for_node(&node, depth));
    world.insert_resource(saved.run);

    world.resource_mut::<NextState<GameState>>().set(GameState::InRun);
}

/// Consumes `ResumeRunRequest` (exclusive, so it can drive the `&mut World` hydration). Runs only
/// on a frame carrying a request; a missing/absent save is a clean no-op (stays wherever it was —
/// the menu — rather than panicking).
pub fn apply_resume_request(world: &mut World) {
    let requested = world.resource_mut::<Events<ResumeRunRequest>>().drain().count() > 0;
    if !requested {
        return;
    }
    let saved = world.resource::<MetaState>().in_progress_run.clone();
    if let Some(saved) = saved {
        resume_run(world, saved);
    }
}

