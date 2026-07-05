// Run reset / restart (Phase 7.5B) — the "tear the run down and start a fresh one" primitive.
//
// This is the missing counterpart to Phase 7's `start_run`: `start_run` assumes a clean world (used
// once at boot), whereas restarting after death — and, from Phase 7.5C, starting a run from
// character-select over an already-spawned world — must first remove every entity/resource the last
// run left behind. Both the death screen (R) and character-select drive it through one
// `StartRunRequest` event so there is a single, tested reset path.
//
// Determinism: `reset_and_start_run` is a pure function of `(seed, hero_id)` — it reseeds `RunRng`
// before both the band-pool shuffle and the graph build — so a restart replays identically under a
// fixed seed (the reproducibility contract, docs/testing.md).
//
// Golden-master note: the campaign never dies, opens a menu, or restarts, and `apply_start_run_request`
// only runs on a frame carrying a `StartRunRequest` (an `on_event` run condition), so this module is
// inert in the campaign ⇒ byte-identical.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use std::collections::HashSet;

use crate::ability::components::AbilityInstance;
use crate::enemy::components::Enemy;
use crate::game::state::{GameOverSummary, GameState};
use crate::hero::assets::{HeroDef, HeroLibrary};
use crate::hero::components::{ActiveStance, HeroIdentity};
use crate::pickup::components::PickUp;
use crate::player::components::Player;
use crate::player::systems::spawn_player::spawn_player;
use crate::progression::systems::level_up::init_level_flow;
use crate::projectile::components::Projectile;
use crate::run::rng::RunRng;
use crate::run::state::{CurrentEncounter, RoomModifiers, RunState};
use crate::run::systems::transitions::{start_run, DEFAULT_RUN_HERO};
use crate::status::components::StatusEffectInstance;
use crate::zone::components::PersistentZone;

/// A request to (re)start a run as `hero_id` with `seed`. Emitted by the death screen (R, Phase
/// 7.5B) and character-select (Phase 7.5C); consumed by `apply_start_run_request`.
#[derive(Event, Debug, Clone)]
pub struct StartRunRequest {
    pub hero_id: String,
    pub seed: u64,
}

/// Full reset: tear down the old run, respawn a fresh level-1 player as `hero_id`, reseed the RNG,
/// re-initialize the level-up flow, and begin a new run at `seed`. A `&mut World` fn (structural
/// resource + entity ops), so it runs as an exclusive system / from the sim harness.
pub fn reset_and_start_run(world: &mut World, seed: u64, hero_id: &str) {
    teardown_run(world);
    // Reseed *before* the band-pool shuffle so the reset is deterministic regardless of the RNG
    // state the previous run left behind. `start_run` reseeds again to the same value for the graph.
    world.insert_resource(RunRng::from_seed(seed));
    respawn_player(world, hero_id);
    let _ = world.run_system_once(init_level_flow); // fresh, seed-shuffled band pools
    world.flush();
    start_run(world, seed, hero_id); // reseeds RunRng(seed), builds the graph + entry encounter
    world.resource_mut::<NextState<GameState>>().set(GameState::InRun);
}

/// Despawns every run-scoped entity (the player included — respawned fresh) and clears the run
/// resources. Ability instances are separate top-level entities (not children of their owner), so
/// they are collected explicitly — nothing else cleans a dead player's or a despawned enemy's.
fn teardown_run(world: &mut World) {
    let mut doomed: HashSet<Entity> = HashSet::new();
    collect::<Enemy>(world, &mut doomed);
    collect::<Projectile>(world, &mut doomed);
    collect::<PersistentZone>(world, &mut doomed);
    collect::<PickUp>(world, &mut doomed);
    collect::<StatusEffectInstance>(world, &mut doomed);
    collect::<AbilityInstance>(world, &mut doomed);
    collect::<Player>(world, &mut doomed);
    for e in doomed {
        world.despawn(e);
    }

    // `start_run` re-inserts RunState + CurrentEncounter; clear the rest so no stale state leaks in.
    world.remove_resource::<CurrentEncounter>();
    world.remove_resource::<RunState>();
    world.remove_resource::<GameOverSummary>();
    if let Some(mut mods) = world.get_resource_mut::<RoomModifiers>() {
        mods.0.clear();
    }
}

fn collect<C: Component>(world: &mut World, out: &mut HashSet<Entity>) {
    let mut q = world.query_filtered::<Entity, With<C>>();
    out.extend(q.iter(world));
}

/// Spawns a fresh player (via the real Startup `spawn_player`) and re-identifies it as `hero_id` with
/// that hero's initial stance (its `stance_a` for stance heroes, else "default"). The deferred
/// level-1 grant re-runs naturally for the new identity on the next frames (`Level1Granted` is absent
/// on the fresh entity).
fn respawn_player(world: &mut World, hero_id: &str) {
    let _ = world.run_system_once(spawn_player);
    world.flush();
    let stance = initial_stance(world, hero_id);
    let mut q = world.query_filtered::<Entity, With<Player>>();
    let Some(player) = q.iter(world).next() else { return };
    if let Some(mut id) = world.get_mut::<HeroIdentity>(player) {
        id.0 = hero_id.to_string();
    }
    if let Some(mut st) = world.get_mut::<ActiveStance>(player) {
        st.0 = stance;
    }
}

/// The stance a freshly selected `hero_id` starts in: `stance_a` for a stance hero (so its LMB/RMB
/// slot mapping resolves immediately), else "default". Falls back to "default" if the HeroDef is not
/// loaded (it always is post-boot).
fn initial_stance(world: &mut World, hero_id: &str) -> String {
    let handle = match world.get_resource::<HeroLibrary>().and_then(|l| l.get(hero_id)) {
        Some(h) => h.clone(),
        None => return "default".to_string(),
    };
    world
        .get_resource::<Assets<HeroDef>>()
        .and_then(|d| d.get(&handle))
        .map(|def| {
            if def.has_stance {
                def.stance_a.clone().unwrap_or_else(|| "default".to_string())
            } else {
                "default".to_string()
            }
        })
        .unwrap_or_else(|| "default".to_string())
}

/// Consumes `StartRunRequest` (exclusive, so it can drive the `&mut World` reset). Runs only on a
/// frame that carries a request (`on_event` run condition) — inert otherwise.
pub fn apply_start_run_request(world: &mut World) {
    let req = world.resource_mut::<Events<StartRunRequest>>().drain().last();
    if let Some(req) = req {
        reset_and_start_run(world, req.seed, &req.hero_id);
    }
}

/// Death-screen input (in `GameState::GameOver`): R restarts the run as the same hero with a fresh
/// seed; M returns to the main menu. Restart is routed through `StartRunRequest` so it shares the one
/// reset path (and the sim can drive that path deterministically).
pub fn handle_game_over_input(
    keys: Res<ButtonInput<KeyCode>>,
    summary: Option<Res<GameOverSummary>>,
    mut requests: EventWriter<StartRunRequest>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        next_state.set(GameState::Menu);
        return;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        let hero_id = summary
            .map(|s| s.hero_id.clone())
            .unwrap_or_else(|| DEFAULT_RUN_HERO.to_string());
        requests.write(StartRunRequest { hero_id, seed: rand::random::<u64>() });
    }
}
