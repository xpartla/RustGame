// Headless simulation harness — runs the full gameplay simulation (GameLogicPlugin) without
// a window, GPU, or renderer. This is the foundation for:
//   - the golden-scenario integration tests in tests/ (backward-compatibility safety net),
//   - the golden-master campaign baseline,
//   - later: batch balance sweeps (hero × build × enemy matrices).
//
// Design notes:
//   - MinimalPlugins + StatesPlugin + AssetPlugin: everything GameLogicPlugin needs, nothing
//     that touches a GPU. Works in WSL, CI, anywhere.
//   - Deterministic time: TimeUpdateStrategy::ManualDuration advances the clock by exactly
//     SIM_DT per update, so N steps always simulate the same duration.
//   - Deterministic RNG: the caller's seed is inserted as RunRng *before* GameLogicPlugin so
//     the plugin's entropy fallback never runs.
//   - Deterministic scheduling: Startup/Update/PostUpdate run single-threaded so ambiguous
//     system pairs execute in a stable order within a build.
//   - Input: ButtonInput resources are initialized manually (no bevy_input plugin), so tests
//     fully control key/button state. Sim::step clears the just_pressed/just_released edge
//     flags after every frame, mirroring what the real input pipeline does.
//   - Remaining nondeterminism: the ambient enemy/pickup spawners intentionally roll
//     rand::thread_rng. Scenario tests pause them (pause_ambient_spawners) and spawn actors
//     explicitly; see docs/testing.md.

use std::time::Duration;

use bevy::app::{App, PluginsState};
use bevy::asset::{AssetPlugin, Assets};
use bevy::ecs::schedule::ExecutorKind;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;

use crate::ability::assets::{AbilityDef, AbilityLibrary};
use crate::ability::components::{AbilityInstance, TriggerAbilityEvent, UnlockAbilityEvent};
use crate::core::components::{Facing, GridPosition, Health, WorldPosition};
use crate::core::events::{DamageEvent, DamageTag};
use crate::enemy::archetypes::{archetypes, EnemyArchetype};
use crate::enemy::components::{Enemy, EnemySpawner};
use crate::game::state::GameState;
use crate::game::GameLogicPlugin;
use crate::pickup::components::PickUpSpawner;
use crate::player::components::Player;
use crate::progression::state::LevelUpFlowState;
use crate::run::rng::RunRng;
use crate::status::assets::{StatusEffectDef, StatusLibrary};
use crate::status::components::{ApplyStatusEvent, StatusEffectInstance};
use crate::talent::assets::{TalentDef, TalentLibrary};
use crate::talent::components::AcquiredTalents;
use crate::world::components::TileMap;

/// Fixed timestep for every simulation frame: 60 updates per simulated second.
pub const SIM_DT: f32 = 1.0 / 60.0;

/// Plugin group for a headless run of the full game simulation.
pub struct SimPlugins {
    pub seed: u64,
}

impl Plugin for SimPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.add_plugins(AssetPlugin::default());

        // Manual input resources — no window, no bevy_input systems. Tests write these.
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();

        // Deterministic clock and RNG.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(SIM_DT)));
        app.insert_resource(RunRng::from_seed(self.seed));

        // Stable execution order for ambiguous system pairs.
        app.edit_schedule(Startup, |s| {
            s.set_executor_kind(ExecutorKind::SingleThreaded);
        });
        app.edit_schedule(Update, |s| {
            s.set_executor_kind(ExecutorKind::SingleThreaded);
        });
        app.edit_schedule(PostUpdate, |s| {
            s.set_executor_kind(ExecutorKind::SingleThreaded);
        });

        app.add_plugins(GameLogicPlugin);
    }
}

/// A headless game simulation with helpers for scripting scenarios and reading state.
pub struct Sim {
    pub app: App,
}

impl Sim {
    /// Builds the sim and runs the first frame (Startup: player spawn, map generation,
    /// level-flow init, asset loads kicked off). Ambient spawners are still live — call
    /// `pause_ambient_spawners` (or use `new_arena`) for deterministic scenarios.
    pub fn new(seed: u64) -> Self {
        let mut app = App::new();
        app.add_plugins(SimPlugins { seed });
        while app.plugins_state() == PluginsState::Adding {
            std::thread::sleep(Duration::from_millis(1));
        }
        app.finish();
        app.cleanup();
        let mut sim = Self { app };
        sim.step(1);
        sim
    }

    /// The standard deterministic test fixture: fixed seed, ambient spawners paused, map
    /// replaced by an empty bordered arena, and all RON assets fully loaded.
    pub fn new_arena(seed: u64) -> Self {
        let mut sim = Self::new(seed);
        sim.pause_ambient_spawners();
        sim.set_empty_arena_map();
        sim.settle_assets(600);
        sim
    }

    // ---------------------------------------------------------------- stepping

    /// Advances the simulation by `frames` fixed-dt updates. Input edge flags
    /// (just_pressed / just_released) last exactly one frame, like a real input pipeline.
    pub fn step(&mut self, frames: usize) {
        for _ in 0..frames {
            self.app.update();
            let world = self.app.world_mut();
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .bypass_change_detection()
                .clear();
            world
                .resource_mut::<ButtonInput<MouseButton>>()
                .bypass_change_detection()
                .clear();
        }
    }

    /// Advances the simulation by (roughly) `secs` of simulated time.
    pub fn step_seconds(&mut self, secs: f32) {
        self.step((secs / SIM_DT).ceil() as usize);
    }

    /// Pumps frames until every id in AbilityLibrary and TalentLibrary resolves to a loaded
    /// asset. Panics after `max_frames` (an asset failed to load — check the RON).
    pub fn settle_assets(&mut self, max_frames: usize) {
        for _ in 0..max_frames {
            if self.assets_loaded() {
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
            self.step(1);
        }
        panic!("assets did not finish loading within {max_frames} frames — RON parse error?");
    }

    fn assets_loaded(&self) -> bool {
        let world = self.app.world();
        let ability_lib = world.resource::<AbilityLibrary>();
        let ability_defs = world.resource::<Assets<AbilityDef>>();
        let talent_lib = world.resource::<TalentLibrary>();
        let talent_defs = world.resource::<Assets<TalentDef>>();
        let status_lib = world.resource::<StatusLibrary>();
        let status_defs = world.resource::<Assets<StatusEffectDef>>();
        ability_lib.defs.values().all(|h| ability_defs.get(h).is_some())
            && talent_lib.defs.values().all(|h| talent_defs.get(h).is_some())
            && status_lib.defs.values().all(|h| status_defs.get(h).is_some())
    }

    // ---------------------------------------------------------------- world access

    pub fn world(&self) -> &World {
        self.app.world()
    }

    pub fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }

    pub fn game_state(&self) -> GameState {
        self.app.world().resource::<State<GameState>>().get().clone()
    }

    // ---------------------------------------------------------------- player

    /// The player entity. Panics if the player does not exist (e.g. after death).
    pub fn player(&mut self) -> Entity {
        self.try_player().expect("no player entity alive")
    }

    pub fn try_player(&mut self) -> Option<Entity> {
        let world = self.app.world_mut();
        let mut query = world.query_filtered::<Entity, With<Player>>();
        query.iter(world).next()
    }

    pub fn player_health(&mut self) -> f32 {
        let player = self.player();
        self.app.world().get::<Health>(player).unwrap().current
    }

    pub fn set_player_health(&mut self, current: f32) {
        let player = self.player();
        self.app.world_mut().get_mut::<Health>(player).unwrap().current = current;
    }

    pub fn player_pos(&mut self) -> Vec2 {
        let player = self.player();
        self.app.world().get::<WorldPosition>(player).unwrap().0
    }

    /// Teleports the player. GridPosition catches up via world_to_grid on the next frame.
    pub fn set_player_pos(&mut self, pos: Vec2) {
        let player = self.player();
        self.app.world_mut().get_mut::<WorldPosition>(player).unwrap().0 = pos;
    }

    /// Sets the player's aim direction directly (the mouse→facing system no-ops headless
    /// because there is no window).
    pub fn set_player_facing(&mut self, dir: Vec2) {
        let player = self.player();
        self.app.world_mut().get_mut::<Facing>(player).unwrap().0 = dir.normalize_or_zero();
    }

    pub fn player_level(&mut self) -> u32 {
        let player = self.player();
        self.app
            .world()
            .get::<crate::player::components::Experience>(player)
            .unwrap()
            .level
    }

    pub fn acquired_talents(&mut self) -> Vec<(String, u8)> {
        let player = self.player();
        self.app
            .world()
            .get::<AcquiredTalents>(player)
            .map(|a| a.entries.clone())
            .unwrap_or_default()
    }

    /// Ability ids the player currently owns an AbilityInstance for.
    pub fn owned_abilities(&mut self) -> Vec<String> {
        let player = self.player();
        let world = self.app.world_mut();
        let mut query = world.query::<&AbilityInstance>();
        query
            .iter(world)
            .filter(|i| i.owner == player)
            .map(|i| i.def_id.clone())
            .collect()
    }

    // ---------------------------------------------------------------- input

    /// Holds a key down (visible as just_pressed for exactly the next frame, pressed until
    /// released).
    pub fn press_key(&mut self, key: KeyCode) {
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }

    pub fn release_key(&mut self, key: KeyCode) {
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
    }

    /// Press + one frame + release: a single key tap.
    pub fn tap_key(&mut self, key: KeyCode) {
        self.press_key(key);
        self.step(1);
        self.release_key(key);
    }

    pub fn press_mouse(&mut self, button: MouseButton) {
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(button);
    }

    pub fn release_mouse(&mut self, button: MouseButton) {
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(button);
    }

    /// Fires an ability directly through the pipeline (bypasses the mouse-input stub).
    pub fn trigger_ability(&mut self, ability_id: &str) {
        let owner = self.player();
        self.app.world_mut().send_event(TriggerAbilityEvent {
            ability_id: ability_id.to_string(),
            owner,
        });
    }

    /// Grants the player an ability instance through the real UnlockAbilityEvent path (used to
    /// hand the not-yet-class-bound demonstrators — Fireblast/Frostbolt/Scratch — to the player).
    /// Step once afterward so `spawn_unlocked_ability` creates the instance.
    pub fn grant_ability(&mut self, ability_id: &str) {
        let owner = self.player();
        self.app.world_mut().send_event(UnlockAbilityEvent {
            ability_id: ability_id.to_string(),
            owner,
        });
    }

    /// Deals direct damage to an entity through the normal DamageEvent → apply_damage chain.
    pub fn deal_damage(&mut self, target: Entity, amount: f32) {
        self.app.world_mut().send_event(DamageEvent {
            target,
            amount,
            source: Entity::PLACEHOLDER,
            tags: vec![],
        });
    }

    /// Deals element-tagged damage (drives status cross-interaction, e.g. a Fire hit that
    /// clears frostbite) without needing a real fire ability.
    pub fn deal_tagged_damage(&mut self, target: Entity, amount: f32, tag: DamageTag) {
        self.app.world_mut().send_event(DamageEvent {
            target,
            amount,
            source: Entity::PLACEHOLDER,
            tags: vec![tag],
        });
    }

    /// World position of any entity (enemies, projectiles, …). None if it has no WorldPosition.
    pub fn entity_pos(&self, entity: Entity) -> Option<Vec2> {
        self.app.world().get::<WorldPosition>(entity).map(|p| p.0)
    }

    // ---------------------------------------------------------------- enemies

    /// Spawns an enemy of the given archetype at a tile, identical to the timed spawner's
    /// output (same enemy_bundle).
    pub fn spawn_enemy(&mut self, archetype: &EnemyArchetype, tile: (i32, i32)) -> Entity {
        let bundle = crate::enemy::systems::spawner::enemy_bundle(
            archetype,
            GridPosition { x: tile.0, y: tile.1 },
        );
        self.app.world_mut().spawn(bundle).id()
    }

    /// Spawns the baseline Grunt archetype at a tile.
    pub fn spawn_grunt(&mut self, tile: (i32, i32)) -> Entity {
        self.spawn_enemy(&archetypes()[0], tile)
    }

    pub fn enemy_count(&mut self) -> usize {
        let world = self.app.world_mut();
        let mut query = world.query_filtered::<Entity, With<Enemy>>();
        query.iter(world).count()
    }

    pub fn enemy_health(&mut self, enemy: Entity) -> Option<f32> {
        self.app.world().get::<Health>(enemy).map(|h| h.current)
    }

    /// Sets any entity's current health (test setup — e.g. a durable dummy for DoT scenarios).
    pub fn set_health(&mut self, entity: Entity, current: f32) {
        if let Some(mut health) = self.app.world_mut().get_mut::<Health>(entity) {
            health.current = current;
        }
    }

    // ---------------------------------------------------------------- status effects

    /// Applies a status effect to `target`, attributed to `source`, through the normal
    /// ApplyStatusEvent → apply_status_effects chain.
    pub fn apply_status(&mut self, target: Entity, source: Entity, effect_id: &str, stacks: u8) {
        self.app.world_mut().send_event(ApplyStatusEvent {
            target,
            source,
            effect_id: effect_id.to_string(),
            stacks,
        });
    }

    /// The status effect ids currently afflicting `target` (one entry per active instance, so
    /// stacks appear multiple times).
    pub fn status_ids_on(&mut self, target: Entity) -> Vec<String> {
        let world = self.app.world_mut();
        let mut query = world.query::<&StatusEffectInstance>();
        query
            .iter(world)
            .filter(|i| i.target == target)
            .map(|i| i.def_id.clone())
            .collect()
    }

    /// Whether `target` currently has at least one instance of `effect_id`.
    pub fn has_status(&mut self, target: Entity, effect_id: &str) -> bool {
        self.status_ids_on(target).iter().any(|id| id == effect_id)
    }

    /// Total active status-effect instances across all entities (golden-master coverage column).
    pub fn active_status_count(&mut self) -> usize {
        let world = self.app.world_mut();
        let mut query = world.query::<&StatusEffectInstance>();
        query.iter(world).count()
    }

    // ---------------------------------------------------------------- environment control

    /// Pauses the ambient enemy/pickup spawner timers — the two remaining thread_rng
    /// consumers — so scenarios only contain explicitly spawned actors.
    pub fn pause_ambient_spawners(&mut self) {
        let world = self.app.world_mut();
        world.resource_mut::<EnemySpawner>().timer.pause();
        world.resource_mut::<PickUpSpawner>().timer.pause();
    }

    /// Replaces the generated map with an empty arena of the same extents: border walls only,
    /// no interior obstacles. The flow field rebuilds from the new map next frame.
    pub fn set_empty_arena_map(&mut self) {
        let mut map = self.app.world_mut().resource_mut::<TileMap>();
        let (hw, hh) = (map.half_width, map.half_height);
        map.blocked.clear();
        for x in -hw..=hw {
            map.blocked.insert(GridPosition { x, y: -hh });
            map.blocked.insert(GridPosition { x, y: hh });
        }
        for y in -hh..=hh {
            map.blocked.insert(GridPosition { x: -hw, y });
            map.blocked.insert(GridPosition { x: hw, y });
        }
    }

    /// Marks a single tile blocked (for wall-collision scenarios).
    pub fn block_tile(&mut self, x: i32, y: i32) {
        self.app
            .world_mut()
            .resource_mut::<TileMap>()
            .blocked
            .insert(GridPosition { x, y });
    }

    /// Stable FNV-1a signature of the blocked-tile set (order-independent: tiles are sorted
    /// before hashing). Used by map-determinism tests and the golden-master baseline.
    pub fn tilemap_signature(&self) -> u64 {
        let map = self.app.world().resource::<TileMap>();
        let mut tiles: Vec<(i32, i32)> = map.blocked.iter().map(|t| (t.x, t.y)).collect();
        tiles.sort_unstable();
        let mut hash: u64 = 0xcbf29ce484222325;
        let mut feed = |v: i32| {
            for b in v.to_le_bytes() {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        };
        for (x, y) in tiles {
            feed(x);
            feed(y);
        }
        hash
    }

    /// Direct access to the level-up flow state (band pools, owed choices, pending offer).
    pub fn level_flow(&self) -> &LevelUpFlowState {
        self.app.world().resource::<LevelUpFlowState>()
    }
}
