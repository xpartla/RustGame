// Encounter lifecycle + act transitions (Phase 7).
//
// The loop, all gated on a live run (`resource_exists::<CurrentEncounter>`) so a runless world never
// touches it:
//   start_run               — seed RunRng, pick the act theme, build the act graph, insert RunState +
//                             the entry CurrentEncounter (a `&mut World` fn, called by the windowed
//                             auto-start and by Sim::start_run).
//   load_encounter          — a one-shot per encounter: generate the room, teleport the player to the
//                             origin, spawn the themed depth-scaled roster (seeded), (7F) apply the
//                             ThroneRoom curse + emit its reward.
//   check_objective         — track KillAll / Survive / KillMapBoss / Rest → EncounterCompleteEvent.
//   handle_encounter_complete — sync player → RunState; ActBoss ⇒ advance the act (or GameOver on
//                             Act 3); otherwise ⇒ enter MapSelect for a branch choice.
//
// Room generation + roster spawning are seed-deterministic (RunRng only — no thread_rng), so a run
// replays identically (the reproducibility contract, docs/testing.md).

use bevy::prelude::*;
use rand::Rng;

use crate::constants::PLAYER_HEALTH;
use crate::core::components::{GridPosition, Health, WorldPosition};
use crate::enemy::assets::{EnemyDef, EnemyLibrary, ThemeDef, ThemeLibrary, THEME_IDS};
use crate::enemy::components::{Enemy, MapBoss};
use crate::enemy::systems::spawner::spawn_enemy_from_def;
use crate::game::state::{GameOverSummary, GameState};
use crate::pickup::components::PickUp;
use crate::player::components::{Experience, Player};
use crate::projectile::components::Projectile;
use crate::run::rng::RunRng;
use crate::run::state::{node_depth, CurrentEncounter, ObjectiveProgress, RoomModifiers, RunState};
use crate::world::components::TileMap;
use crate::world::constants::SPAWN_CLEAR_RADIUS;
use crate::world::generator::generate_room;
use crate::world::graph::{build_act_graph, EncounterType, ObjectiveType};
use crate::zone::components::PersistentZone;

/// The default hero a fresh run starts as (until character-select lands in Phase 8).
pub const DEFAULT_RUN_HERO: &str = "blood_death_knight";

/// Emitted by `check_objective` the frame an encounter's objective is met.
#[derive(Event, Debug)]
pub struct EncounterCompleteEvent;

// ── Run start ────────────────────────────────────────────────────────────────────────────────

/// Begins a run: reseed RunRng from `seed`, pick the act-1 theme, build the act graph, and insert
/// RunState + the entry CurrentEncounter (un-spawned — `load_encounter` fills it next frame). A
/// `&mut World` fn so both the windowed Startup auto-start and Sim::start_run can drive it.
pub fn start_run(world: &mut World, seed: u64, hero_id: &str) {
    world.insert_resource(RunRng::from_seed(seed));

    let (theme, graph) = {
        let mut rng = world.resource_mut::<RunRng>();
        let rng = &mut *rng;
        let theme = THEME_IDS[rng.rng().gen_range(0..THEME_IDS.len())].to_string();
        let graph = build_act_graph(1, theme.clone(), rng);
        (theme, graph)
    };
    let _ = theme;

    let entry_id = graph.entry;
    let entry = graph.node(entry_id).expect("graph has an entry node").clone();

    let player_health = {
        let mut q = world.query_filtered::<&Health, With<Player>>();
        q.iter(world).next().map(|h| h.current).unwrap_or(PLAYER_HEALTH)
    };
    // Cloned for Phase-8 serialization only (unused during Phase-7 play). Tolerate its absence so the
    // windowed PostStartup auto-start never races the deferred `init_level_flow` insert.
    let level_flow = world
        .get_resource::<crate::progression::state::LevelUpFlowState>()
        .cloned()
        .unwrap_or_else(|| crate::progression::state::LevelUpFlowState::new(Vec::new(), Vec::new()));

    world.insert_resource(RunState {
        seed,
        hero_id: hero_id.to_string(),
        current_act: 1,
        current_node: entry_id,
        act_graph: graph,
        player_health,
        player_level: 1,
        unlocked_abilities: Vec::new(),
        acquired_talents: Vec::new(),
        level_flow,
    });

    let depth = node_depth(1, entry.column);
    world.insert_resource(CurrentEncounter::for_node(&entry, depth));
}

// ── Encounter load ───────────────────────────────────────────────────────────────────────────

/// One-shot per encounter (guarded by `spawned`): generate the room, teleport the player to the
/// origin, and spawn the seeded, depth-scaled, themed roster. Defers (retries next frame) until the
/// themed content has loaded, so the windowed async asset load never spawns an empty roster.
#[allow(clippy::too_many_arguments)]
pub fn load_encounter(
    mut commands: Commands,
    mut current: ResMut<CurrentEncounter>,
    mut map: ResMut<TileMap>,
    mut rng: ResMut<RunRng>,
    mut room_mods: ResMut<RoomModifiers>,
    mut rewards: EventWriter<crate::progression::systems::offer::ThroneRoomRewardEvent>,
    theme_lib: Res<ThemeLibrary>,
    theme_defs: Res<Assets<ThemeDef>>,
    enemy_lib: Res<EnemyLibrary>,
    enemy_defs: Res<Assets<EnemyDef>>,
    modifier_lib: Res<crate::world::graph::RoomModifierLibrary>,
    modifier_defs: Res<Assets<crate::world::graph::RoomModifierDef>>,
    mut players: Query<(Entity, &mut WorldPosition, &mut GridPosition), With<Player>>,
) {
    if current.spawned {
        return;
    }
    // Defer until the enemy defs (and this node's theme) have loaded — else spawn nothing yet.
    let enemies_ready = enemy_lib.defs.values().all(|h| enemy_defs.get(h).is_some());
    let theme_ready = current
        .theme
        .as_ref()
        .map_or(true, |t| theme_lib.get(t).and_then(|h| theme_defs.get(h)).is_some());
    if !enemies_ready || !theme_ready {
        return;
    }

    let encounter = current.encounter.clone();
    let theme_id = current.theme.clone();
    let modifier_id = current.modifier.clone();
    let depth = current.depth;

    // 1. Room geometry.
    generate_room(&encounter, &mut map, &mut rng);

    // 2. Teleport the player to the safe spawn (origin).
    let player = players.iter().next().map(|(e, _, _)| e);
    for (_, mut pos, mut grid) in &mut players {
        pos.0 = Vec2::ZERO;
        *grid = GridPosition { x: 0, y: 0 };
    }

    // 3. ThroneRoom curse (7F): populate RoomModifiers; kiss: emit the reward. Cleared for every
    // non-ThroneRoom encounter so the curse lasts exactly this fight.
    room_mods.0.clear();
    if let (EncounterType::ThroneRoom, Some(mod_id)) = (&encounter, &modifier_id) {
        if let Some(def) = modifier_lib.get(mod_id).and_then(|h| modifier_defs.get(h)) {
            room_mods.0 = def.curse_modifiers.clone();
        }
        if let Some(owner) = player {
            rewards.write(crate::progression::systems::offer::ThroneRoomRewardEvent { owner });
        }
    }

    // 4. Roster (seeded, depth-scaled, themed).
    let theme_def = theme_id
        .as_ref()
        .and_then(|t| theme_lib.get(t))
        .and_then(|h| theme_defs.get(h));
    spawn_roster(&mut commands, &encounter, theme_def, depth, &map, &mut rng, &enemy_lib, &enemy_defs);

    current.spawned = true;
}

/// Spawns the encounter's roster: a themed pack (+ a tagged `MapBoss` for KillMapBoss), a single boss
/// for BossRoom/ActBoss, nothing for Merchant. Every spawn goes through `spawn_enemy_from_def(.., depth)`
/// so the Phase-5 scaling curve (health/xp + a DamageDealtModifier) is finally driven. Weighted-picks
/// from the theme pools via RunRng (seed-deterministic). An unknown/unloaded id degrades gracefully.
#[allow(clippy::too_many_arguments)]
fn spawn_roster(
    commands: &mut Commands,
    encounter: &EncounterType,
    theme: Option<&ThemeDef>,
    depth: u32,
    map: &TileMap,
    rng: &mut RunRng,
    enemy_lib: &EnemyLibrary,
    enemy_defs: &Assets<EnemyDef>,
) {
    let pack_count = (4 + depth / 2).min(14) as usize;
    match encounter {
        EncounterType::Merchant => {}
        EncounterType::ActBoss => {
            // No theme on an act boss node → the placeholder warlord is the act boss.
            spawn_enemy(commands, "warlord", depth, map, rng, enemy_lib, enemy_defs, true);
        }
        EncounterType::BossRoom => {
            if let Some(id) = theme.and_then(|t| pick_id(&t.boss_pool, rng)) {
                spawn_enemy(commands, &id, depth, map, rng, enemy_lib, enemy_defs, true);
            }
        }
        EncounterType::ThroneRoom => {
            for _ in 0..pack_count {
                spawn_pack(commands, theme, depth, map, rng, enemy_lib, enemy_defs);
            }
        }
        EncounterType::Map { objective } => {
            for _ in 0..pack_count {
                spawn_pack(commands, theme, depth, map, rng, enemy_lib, enemy_defs);
            }
            if let ObjectiveType::KillMapBoss { .. } = objective {
                if let Some(id) = theme.and_then(|t| pick_id(&t.map_boss_pool, rng)) {
                    spawn_enemy(commands, &id, depth, map, rng, enemy_lib, enemy_defs, true);
                }
            }
        }
    }
}

/// Spawns one pack enemy weighted-picked from the theme's common pool.
#[allow(clippy::too_many_arguments)]
fn spawn_pack(
    commands: &mut Commands,
    theme: Option<&ThemeDef>,
    depth: u32,
    map: &TileMap,
    rng: &mut RunRng,
    enemy_lib: &EnemyLibrary,
    enemy_defs: &Assets<EnemyDef>,
) {
    if let Some(id) = theme.and_then(|t| pick_id(&t.common_enemy_pool, rng)) {
        spawn_enemy(commands, &id, depth, map, rng, enemy_lib, enemy_defs, false);
    }
}

/// Resolves an enemy id and spawns it on a free ring tile; `boss` tags it `MapBoss`. Skips + warns if
/// the id is unknown/unloaded (graceful degradation — a bad theme pool never panics).
#[allow(clippy::too_many_arguments)]
fn spawn_enemy(
    commands: &mut Commands,
    id: &str,
    depth: u32,
    map: &TileMap,
    rng: &mut RunRng,
    enemy_lib: &EnemyLibrary,
    enemy_defs: &Assets<EnemyDef>,
    boss: bool,
) {
    let Some(def) = enemy_lib.get(id).and_then(|h| enemy_defs.get(h)) else {
        warn!("encounter roster references unknown/unloaded enemy '{id}' — skipping");
        return;
    };
    let radius = if boss { 5.0 } else { 9.0 };
    let grid = free_ring_tile(map, rng, radius);
    let entity = spawn_enemy_from_def(commands, def, grid, depth);
    if boss {
        commands.entity(entity).insert(MapBoss);
    }
}

/// Uniformly picks an id from a pool via RunRng (None if empty).
fn pick_id(pool: &[String], rng: &mut RunRng) -> Option<String> {
    if pool.is_empty() {
        return None;
    }
    Some(pool[rng.rng().gen_range(0..pool.len())].clone())
}

/// A walkable tile on a ring of `radius` tiles around the origin (retries a few angles off blocked
/// tiles; falls back to the always-clear spawn-box edge).
fn free_ring_tile(map: &TileMap, rng: &mut RunRng, radius: f32) -> GridPosition {
    for _ in 0..16 {
        let angle = rng.rng().gen_range(0.0..std::f32::consts::TAU);
        let g = GridPosition {
            x: (angle.cos() * radius).round() as i32,
            y: (angle.sin() * radius).round() as i32,
        };
        if !map.is_blocked(g) {
            return g;
        }
    }
    GridPosition { x: SPAWN_CLEAR_RADIUS, y: 0 }
}

// ── Objective tracking ─────────────────────────────────────────────────────────────────────────

/// Tracks the live objective and emits `EncounterCompleteEvent` when it is met. Runs in
/// `CombatSet::Death` after `enemy_death`, so a killed enemy/boss is already despawned before we count.
pub fn check_objective(
    time: Res<Time>,
    mut current: ResMut<CurrentEncounter>,
    enemies: Query<(), With<Enemy>>,
    bosses: Query<(), With<MapBoss>>,
    mut complete: EventWriter<EncounterCompleteEvent>,
) {
    if !current.spawned {
        return;
    }
    let enemy_n = enemies.iter().count();
    let boss_n = bosses.iter().count();

    // Arm a kill objective only once its targets are observed present (guards the spawn-frame gap).
    match current.objective {
        ObjectiveProgress::KillAll if enemy_n > 0 => current.armed = true,
        ObjectiveProgress::KillMapBoss if boss_n > 0 => current.armed = true,
        _ => {}
    }
    let armed = current.armed;

    let finished = match &mut current.objective {
        ObjectiveProgress::KillAll => armed && enemy_n == 0,
        ObjectiveProgress::KillMapBoss => armed && boss_n == 0,
        ObjectiveProgress::Survive { timer } => {
            timer.tick(time.delta());
            timer.finished()
        }
        // A Merchant `Rest` no longer auto-completes (Phase 7.5E): `enter_merchant` opens the shop
        // overlay, and the player leaves it directly to MapSelect. So it never completes via the
        // objective path.
        ObjectiveProgress::Rest => false,
    };
    if finished {
        complete.write(EncounterCompleteEvent);
    }
}

/// Opens the merchant shop when a Merchant node has loaded (Phase 7.5E). Transitions InRun → Merchant
/// once the (empty) room is spawned; the shop overlay's input leaves directly to MapSelect, so this
/// never re-fires for the same node (it is InRun-gated and the node is left via MapSelect, not InRun).
pub fn enter_merchant(
    current: Res<CurrentEncounter>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if current.spawned && matches!(current.objective, ObjectiveProgress::Rest) {
        next_state.set(GameState::Merchant);
    }
}

// ── Advance ──────────────────────────────────────────────────────────────────────────────────

/// Consumes `EncounterCompleteEvent`: sync the player into RunState, then either advance the act (an
/// ActBoss clear) — rebuilding the next act's graph, or ending the run on Act 3 — or enter MapSelect
/// for a branch choice (every other encounter).
#[allow(clippy::too_many_arguments)]
pub fn handle_encounter_complete(
    mut events: EventReader<EncounterCompleteEvent>,
    mut commands: Commands,
    current: Res<CurrentEncounter>,
    mut run_state: ResMut<RunState>,
    mut next_state: ResMut<NextState<GameState>>,
    mut rng: ResMut<RunRng>,
    mut room_mods: ResMut<RoomModifiers>,
    players: Query<(&Health, &Experience), With<Player>>,
    enemies: Query<Entity, With<Enemy>>,
    projectiles: Query<Entity, With<Projectile>>,
    zones: Query<Entity, With<PersistentZone>>,
    pickups: Query<Entity, With<PickUp>>,
) {
    if events.is_empty() {
        return;
    }
    events.clear();

    // Sync the live player into RunState (Phase-8 serialization; player entity stays authoritative).
    if let Ok((health, exp)) = players.single() {
        run_state.player_health = health.current;
        run_state.player_level = exp.level;
    }

    // Key off the encounter that was actually loaded (CurrentEncounter), not a graph re-lookup.
    let is_act_boss = matches!(current.encounter, EncounterType::ActBoss);

    if is_act_boss {
        despawn_encounter_entities(&mut commands, &enemies, &projectiles, &zones, &pickups);
        room_mods.0.clear();
        if run_state.current_act >= 3 {
            // Run complete (Act-3 boss down) — a victory. Capture the summary before teardown so the
            // game-over screen can render it (Phase 7.5B); Phase 8 also records the score / RunRecord.
            commands.insert_resource(GameOverSummary {
                victory: true,
                hero_id: run_state.hero_id.clone(),
                level: run_state.player_level,
                act: Some(run_state.current_act),
                node_column: run_state.act_graph.node(run_state.current_node).map(|n| n.column),
            });
            commands.remove_resource::<CurrentEncounter>();
            commands.remove_resource::<RunState>();
            next_state.set(GameState::GameOver);
        } else {
            run_state.current_act += 1;
            let act = run_state.current_act;
            let theme = THEME_IDS[rng.rng().gen_range(0..THEME_IDS.len())].to_string();
            let graph = build_act_graph(act, theme, &mut rng);
            let entry_id = graph.entry;
            let entry = graph.node(entry_id).expect("new act graph has an entry").clone();
            run_state.act_graph = graph;
            run_state.current_node = entry_id;
            let depth = node_depth(act, entry.column);
            commands.insert_resource(CurrentEncounter::for_node(&entry, depth));
            // Stay InRun — load_encounter loads the new act's entry next frame.
        }
    } else {
        next_state.set(GameState::MapSelect);
    }
}

/// Despawns exactly the encounter-scoped entities (enemies, projectiles/VFX, zones, pickups) on an
/// encounter transition. The player entity persists across encounters. Status-effect instances on a
/// despawned enemy are reaped by `despawn_orphaned_status` next frame.
pub fn despawn_encounter_entities(
    commands: &mut Commands,
    enemies: &Query<Entity, With<Enemy>>,
    projectiles: &Query<Entity, With<Projectile>>,
    zones: &Query<Entity, With<PersistentZone>>,
    pickups: &Query<Entity, With<PickUp>>,
) {
    for e in enemies.iter() {
        commands.entity(e).despawn();
    }
    for e in projectiles.iter() {
        commands.entity(e).despawn();
    }
    for e in zones.iter() {
        commands.entity(e).despawn();
    }
    for e in pickups.iter() {
        commands.entity(e).despawn();
    }
}
