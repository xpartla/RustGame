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
use crate::ability::components::{AbilityCooldown, AbilityInstance, Level1Granted, TriggerAbilityEvent, UnlockAbilityEvent};
use crate::core::components::{Absorb, DamageDealtModifier, Facing, Faction, ForcedImpulse, GridPosition, Health, WorldPosition};
use crate::core::events::{DamageEvent, DamageTag, GainShieldEvent};
use crate::enemy::assets::{resolve_enemy_stats, EnemyDef, EnemyLibrary, ThemeDef, ThemeLibrary};
use crate::enemy::components::{Enemy, EnemySpawner};
use crate::enemy::systems::spawner::enemy_bundle;
use crate::game::state::GameState;
use crate::game::GameLogicPlugin;
use crate::hero::assets::{HeroDef, HeroLibrary};
use crate::hero::components::{ActiveStance, Charges, ClassResource, HeroIdentity};
use crate::pickup::components::PickUpSpawner;
use crate::player::components::Player;
use crate::progression::state::LevelUpFlowState;
use crate::run::rng::RunRng;
use crate::run::state::{CurrentEncounter, RoomModifiers, RunState};
use crate::world::graph::{EncounterNode, EncounterType, NodeId};
use crate::status::assets::{StatusEffectDef, StatusLibrary};
use crate::status::components::{ApplyStatusEvent, StatusEffectInstance};
use crate::talent::assets::{TalentDef, TalentLibrary};
use crate::talent::components::AcquiredTalents;
use crate::world::components::TileMap;
use crate::world::graph::{RoomModifierDef, RoomModifierLibrary};
use crate::zone::components::{PersistentZone, PlayerZonePresence, ZoneAnchor};

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

    /// Pumps frames until every def library resolves to loaded assets AND the hero-driven
    /// level-1 grant has run for every spawned player. Panics after `max_frames` (an asset failed
    /// to load — check the RON).
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

    fn assets_loaded(&mut self) -> bool {
        let libs_ready = {
            let world = self.app.world();
            let ability_lib = world.resource::<AbilityLibrary>();
            let ability_defs = world.resource::<Assets<AbilityDef>>();
            let talent_lib = world.resource::<TalentLibrary>();
            let talent_defs = world.resource::<Assets<TalentDef>>();
            let status_lib = world.resource::<StatusLibrary>();
            let status_defs = world.resource::<Assets<StatusEffectDef>>();
            let hero_lib = world.resource::<HeroLibrary>();
            let hero_defs = world.resource::<Assets<HeroDef>>();
            let enemy_lib = world.resource::<EnemyLibrary>();
            let enemy_defs = world.resource::<Assets<EnemyDef>>();
            let theme_lib = world.resource::<ThemeLibrary>();
            let theme_defs = world.resource::<Assets<ThemeDef>>();
            let modifier_lib = world.resource::<RoomModifierLibrary>();
            let modifier_defs = world.resource::<Assets<RoomModifierDef>>();
            ability_lib.defs.values().all(|h| ability_defs.get(h).is_some())
                && talent_lib.defs.values().all(|h| talent_defs.get(h).is_some())
                && status_lib.defs.values().all(|h| status_defs.get(h).is_some())
                && hero_lib.defs.values().all(|h| hero_defs.get(h).is_some())
                && enemy_lib.defs.values().all(|h| enemy_defs.get(h).is_some())
                && theme_lib.defs.values().all(|h| theme_defs.get(h).is_some())
                && modifier_lib.defs.values().all(|h| modifier_defs.get(h).is_some())
        };
        if !libs_ready {
            return false;
        }
        // The hero-driven level-1 grant is deferred until the HeroDef asset loads
        // (ability/plugin.rs::grant_level_1_abilities). Keep pumping until no player is still
        // awaiting its grant, so new_arena(...) returns with starting abilities in place.
        let world = self.app.world_mut();
        let mut ungranted =
            world.query_filtered::<Entity, (With<Player>, Without<Level1Granted>)>();
        ungranted.iter(world).next().is_none()
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

    // ---------------------------------------------------------------- hero / stance

    /// Re-identifies an entity as `hero_id` in `stance` and clears the Level1Granted marker so the
    /// deferred grant re-runs for the new class. Step at least once afterward so the grant fires
    /// (fireblast/frostbolt for the Mage) — mirrors what the debug hotkey does at runtime.
    pub fn set_hero(&mut self, entity: Entity, hero_id: &str, stance: &str) {
        let world = self.app.world_mut();
        if let Some(mut id) = world.get_mut::<HeroIdentity>(entity) {
            id.0 = hero_id.to_string();
        }
        if let Some(mut st) = world.get_mut::<ActiveStance>(entity) {
            st.0 = stance.to_string();
        }
        world.entity_mut(entity).remove::<Level1Granted>();
    }

    /// The player's current hero id (HeroIdentity).
    pub fn hero_id(&mut self) -> String {
        let player = self.player();
        self.app
            .world()
            .get::<HeroIdentity>(player)
            .map(|h| h.0.clone())
            .unwrap_or_default()
    }

    /// The player's current active stance (ActiveStance).
    pub fn active_stance(&mut self) -> String {
        let player = self.player();
        self.app
            .world()
            .get::<ActiveStance>(player)
            .map(|s| s.0.clone())
            .unwrap_or_default()
    }

    /// Test-only: binds every `stance_slots` entry of the loaded `hero_id`'s `HeroDef` to
    /// `ability_id` on its `movement` slot (mutates the loaded asset in place, mirroring
    /// `set_ability_param`'s override pattern). No shipped hero binds `movement` yet (Phase 9.1);
    /// this exercises the Shift/Space input path end-to-end without waiting for the class that
    /// eventually claims the slot.
    pub fn bind_movement_ability(&mut self, hero_id: &str, ability_id: &str) {
        let world = self.app.world_mut();
        let handle = world
            .resource::<HeroLibrary>()
            .get(hero_id)
            .unwrap_or_else(|| panic!("unknown hero '{hero_id}'"))
            .clone();
        let mut defs = world.resource_mut::<Assets<HeroDef>>();
        let def = defs
            .get_mut(&handle)
            .unwrap_or_else(|| panic!("hero '{hero_id}' not loaded yet — settle assets first"));
        for slot in &mut def.stance_slots {
            slot.movement = Some(ability_id.to_string());
        }
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

    /// Press + one frame + release: a single mouse-button tap (mirrors `tap_key`). The press is
    /// visible as `just_pressed` for exactly the stepped frame, so input-driven casts fire once.
    pub fn tap_mouse(&mut self, button: MouseButton) {
        self.press_mouse(button);
        self.step(1);
        self.release_mouse(button);
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

    /// Spawns an enemy by `EnemyDef` id at a tile (depth 0), identical to the timed spawner's
    /// output — the enemy plus its ability instances (contact melee, etc.). Panics if the id is
    /// unknown or the def has not loaded (settle assets first).
    pub fn spawn_enemy(&mut self, id: &str, tile: (i32, i32)) -> Entity {
        self.spawn_enemy_def(id, tile, 0)
    }

    /// Spawns an enemy at a scaling `depth` (Phase 5, data-only). Depth 0 == base stats; depth > 0
    /// scales health/xp and inserts a `DamageDealtModifier` — for balance/scaling scenarios.
    pub fn spawn_enemy_at_depth(&mut self, id: &str, tile: (i32, i32), depth: u32) -> Entity {
        self.spawn_enemy_def(id, tile, depth)
    }

    /// Spawns the baseline Grunt at a tile.
    pub fn spawn_grunt(&mut self, tile: (i32, i32)) -> Entity {
        self.spawn_enemy("grunt", tile)
    }

    /// Core enemy spawn: resolves the `EnemyDef`, spawns the bundle + one AbilityInstance per
    /// declared ability, and (depth > 0) a `DamageDealtModifier`. Mirrors `spawn_enemy_from_def`
    /// but on the World (the sim has direct world access, not a Commands buffer).
    fn spawn_enemy_def(&mut self, id: &str, tile: (i32, i32), depth: u32) -> Entity {
        let def = {
            let world = self.app.world();
            let handle = world
                .resource::<EnemyLibrary>()
                .get(id)
                .unwrap_or_else(|| panic!("unknown enemy '{id}'"))
                .clone();
            world
                .resource::<Assets<EnemyDef>>()
                .get(&handle)
                .unwrap_or_else(|| panic!("enemy '{id}' not loaded yet — settle assets first"))
                .clone()
        };
        let grid = GridPosition { x: tile.0, y: tile.1 };
        let stats = resolve_enemy_stats(&def, depth);
        let world = self.app.world_mut();
        let enemy = world.spawn(enemy_bundle(&def, grid, depth)).id();
        if (stats.damage_mult - 1.0).abs() > 1e-6 {
            world.entity_mut(enemy).insert(DamageDealtModifier(stats.damage_mult));
        }
        for ability_id in &def.abilities {
            world.spawn((
                AbilityInstance { def_id: ability_id.clone(), owner: enemy },
                AbilityCooldown::new(0.0),
            ));
        }
        enemy
    }

    /// The AbilityIds of the instances owned by `enemy` (contact melee, ranged bolt, …).
    pub fn enemy_ability_ids(&mut self, enemy: Entity) -> Vec<String> {
        let world = self.app.world_mut();
        let mut query = world.query::<&AbilityInstance>();
        query
            .iter(world)
            .filter(|i| i.owner == enemy)
            .map(|i| i.def_id.clone())
            .collect()
    }

    /// An entity's `Faction`, if any (player = Friendly, enemies = Hostile).
    pub fn faction(&self, entity: Entity) -> Option<Faction> {
        self.app.world().get::<Faction>(entity).copied()
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

    /// Registers a synthetic StatusEffectDef directly in the library (no RON file) — for
    /// exercising stacking rules no shipped effect uses yet (StackCapped, StackUnlimited).
    pub fn insert_status_def(&mut self, def: StatusEffectDef) {
        let id = def.id.clone();
        let handle = self
            .app
            .world_mut()
            .resource_mut::<Assets<StatusEffectDef>>()
            .add(def);
        self.app
            .world_mut()
            .resource_mut::<StatusLibrary>()
            .defs
            .insert(id, handle);
    }

    /// Fast-forwards the DoT tick timer of `target`'s `effect_id` instance so its next tick
    /// fires on the very next stepped frame. Lets scenarios align a tick with another event
    /// (e.g. the frame a level-up opens the TalentPicker) without fragile frame counting.
    pub fn hasten_status_tick(&mut self, target: Entity, effect_id: &str) {
        let dt = Duration::from_secs_f32(SIM_DT);
        let world = self.app.world_mut();
        let mut query = world.query::<&mut StatusEffectInstance>();
        for mut inst in query.iter_mut(world) {
            if inst.target == target && inst.def_id == effect_id {
                if let Some(tick_timer) = inst.tick_timer.as_mut() {
                    let target_elapsed = tick_timer.duration().saturating_sub(dt);
                    tick_timer.set_elapsed(target_elapsed);
                }
            }
        }
    }

    /// Overrides one base param of a loaded AbilityDef in place (e.g. give Frostbolt pierce).
    /// Test-only knob: the change lasts for this sim's lifetime and bypasses the talent stack.
    pub fn set_ability_param(&mut self, ability_id: &str, key: &str, value: f32) {
        let world = self.app.world_mut();
        let handle = world
            .resource::<AbilityLibrary>()
            .get(ability_id)
            .unwrap_or_else(|| panic!("unknown ability '{ability_id}'"))
            .clone();
        let mut defs = world.resource_mut::<Assets<AbilityDef>>();
        let def = defs
            .get_mut(&handle)
            .unwrap_or_else(|| panic!("ability '{ability_id}' not loaded yet — settle assets first"));
        def.base_params.insert(key.to_string(), value);
    }

    /// The `ActiveHooks` currently installed on the player (one per Behavior talent acquired). Used
    /// to assert a merchant remove pops the hook.
    pub fn active_hooks(&mut self) -> Vec<String> {
        let player = self.player();
        self.app
            .world()
            .get::<crate::talent::components::ActiveHooks>(player)
            .map(|h| h.hooks.clone())
            .unwrap_or_default()
    }

    /// Emits a `MerchantRemoveRequest` for the player (the merchant remove op). Step once afterward so
    /// the handler + `uninstall_removed_talent` run.
    pub fn merchant_remove(&mut self, talent_id: &str) {
        let owner = self.player();
        self.app.world_mut().send_event(
            crate::talent::systems::merchant::MerchantRemoveRequest {
                owner,
                talent_id: talent_id.to_string(),
            },
        );
    }

    /// Emits a `MerchantTradeRequest` sacrificing three talents (the 3-for-1 trade op). Step
    /// afterward so the trade → TradeUpRewardEvent → Rare-floored picker chain runs.
    pub fn merchant_trade(&mut self, sacrifice: [&str; 3]) {
        let owner = self.player();
        self.app.world_mut().send_event(
            crate::talent::systems::merchant::MerchantTradeRequest {
                owner,
                sacrifice: [sacrifice[0].to_string(), sacrifice[1].to_string(), sacrifice[2].to_string()],
            },
        );
    }

    /// The talent ids currently in the pending offer (the TalentPicker options), for asserting a
    /// trade-up's rarity floor.
    pub fn pending_offer_ids(&self) -> Vec<String> {
        self.app
            .world()
            .get_resource::<LevelUpFlowState>()
            .and_then(|f| f.pending_offer.as_ref().map(|o| o.options.clone()))
            .unwrap_or_default()
    }

    /// The rarity of a loaded talent def (`"Common"`/`"Rare"`/`"Epic"`), for offer-floor assertions.
    pub fn talent_rarity(&self, talent_id: &str) -> Option<String> {
        let world = self.app.world();
        let handle = world.resource::<TalentLibrary>().get(talent_id)?.clone();
        world
            .resource::<Assets<TalentDef>>()
            .get(&handle)
            .map(|d| format!("{:?}", d.rarity))
    }

    /// Installs a talent on the player through the real TalentAcquiredEvent path (adds to
    /// AcquiredTalents; a `Behavior` talent also installs its `ActiveHook`). Step once afterward so
    /// `install_acquired_talent` runs. Hands a talent to the player without an offer — e.g. the
    /// scenario-only `blood_boil_dnd_range_rare`, which is kept out of the offerable pool.
    pub fn grant_talent(&mut self, talent_id: &str) {
        let owner = self.player();
        self.app
            .world_mut()
            .send_event(crate::talent::systems::apply::TalentAcquiredEvent {
                owner,
                talent_id: talent_id.to_string(),
            });
    }

    /// Awards XP to the player through the normal GainXpEvent path (kills, scripted surges).
    pub fn grant_xp(&mut self, amount: u32) {
        let target = self.player();
        self.app
            .world_mut()
            .send_event(crate::core::events::GainXpEvent { target, amount });
    }

    // ---------------------------------------------------------------- shields (Phase 9.1)

    /// Grants `amount` of absorb shield to `target` through the real `GainShieldEvent` path. Step
    /// afterward so `apply_shield_gain` runs.
    pub fn give_shield(&mut self, target: Entity, amount: f32) {
        self.app.world_mut().send_event(GainShieldEvent { target, amount });
    }

    /// The live `Absorb` pool on `entity` (0.0 if it carries none — matches how a fully-drained
    /// shield reads once `apply_damage` removes the component).
    pub fn shield_amount(&self, entity: Entity) -> f32 {
        self.app.world().get::<Absorb>(entity).map(|a| a.amount).unwrap_or(0.0)
    }

    // ---------------------------------------------------------------- forced movement (Phase 9.1)

    /// Inserts a `ForcedImpulse` on `entity` pulling it toward `target` at `speed` for `duration`
    /// seconds (test tool for the grip primitive — bypasses the ability path; Abomination Limb wires
    /// this to a real cast in Phase 9.2). The direction is resolved once from the entity's current
    /// position.
    pub fn pull_toward(&mut self, entity: Entity, target: Vec2, speed: f32, duration: f32) {
        let from = self.entity_pos(entity).unwrap_or(target);
        let impulse = ForcedImpulse::toward_point(from, target, speed, duration);
        self.app.world_mut().entity_mut(entity).insert(impulse);
    }

    /// Inserts a `ForcedImpulse` on `entity` pushing it along `direction` at `speed` for `duration`
    /// seconds (test tool for the knockback primitive).
    pub fn knockback(&mut self, entity: Entity, direction: Vec2, speed: f32, duration: f32) {
        let impulse = ForcedImpulse::knockback(direction, speed, duration);
        self.app.world_mut().entity_mut(entity).insert(impulse);
    }

    // ---------------------------------------------------------------- class resource (Phase 9.1)

    /// Directly sets `entity`'s `Charges` component (test tool — bypasses the ability path; Mage
    /// frost charges / Druid combo charges are the first real producers, Phase 9.4/9.5).
    pub fn set_charges(&mut self, entity: Entity, current: u32, max: u32) {
        self.app.world_mut().entity_mut(entity).insert(Charges { current, max });
    }

    /// The `(current, max)` of `entity`'s `ClassResource` bar (the HUD's data source), if present.
    pub fn class_resource(&mut self, entity: Entity) -> Option<(f32, f32)> {
        self.app.world().get::<ClassResource>(entity).map(|r| (r.current, r.max))
    }

    // ---------------------------------------------------------------- zones (Phase 6)

    /// Number of live `PersistentZone` entities in the world.
    pub fn zone_count(&mut self) -> usize {
        let world = self.app.world_mut();
        let mut q = world.query::<&PersistentZone>();
        q.iter(world).count()
    }

    /// The `zone_type` of every live zone (one entry per zone entity).
    pub fn zone_types(&mut self) -> Vec<String> {
        let world = self.app.world_mut();
        let mut q = world.query::<&PersistentZone>();
        q.iter(world).map(|z| z.zone_type.clone()).collect()
    }

    /// World-space centre of the first live zone of `zone_type`, if any (Fixed anchor value or the
    /// Follow-updated WorldPosition — both live on the entity).
    pub fn zone_center(&mut self, zone_type: &str) -> Option<Vec2> {
        let world = self.app.world_mut();
        let mut q = world.query::<(&PersistentZone, &WorldPosition)>();
        q.iter(world)
            .find(|(z, _)| z.zone_type == zone_type)
            .map(|(_, pos)| pos.0)
    }

    /// Directly spawns a `PersistentZone` (test tool — bypasses the ability path). `follow: Some(e)`
    /// makes it track entity `e` (ZoneAnchor::Follow, the AMZ-epic mechanism); `None` fixes it at
    /// `center`. A marker zone (no ZoneEffects / blocking) carrying `faction`.
    pub fn spawn_zone(
        &mut self,
        zone_type: &str,
        center: Vec2,
        radius: f32,
        duration: f32,
        follow: Option<Entity>,
        faction: Faction,
    ) -> Entity {
        let anchor = match follow {
            Some(e) => ZoneAnchor::Follow(e),
            None => ZoneAnchor::Fixed(center),
        };
        self.app
            .world_mut()
            .spawn((
                PersistentZone {
                    zone_type: zone_type.to_string(),
                    owner: Entity::PLACEHOLDER,
                    radius,
                    duration: Timer::from_seconds(duration, TimerMode::Once),
                    anchor,
                },
                WorldPosition(center),
                faction,
            ))
            .id()
    }

    /// Whether the player currently stands inside a zone of `zone_type` — reads
    /// `PlayerZonePresence`, the exact cache gameplay systems read (rebuilt each frame).
    pub fn player_in_zone(&self, zone_type: &str) -> bool {
        self.app
            .world()
            .resource::<PlayerZonePresence>()
            .is_inside(zone_type)
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

    // ---------------------------------------------------------------- run / encounters (Phase 7)

    /// Begins a run from `seed` (reseeds RunRng, builds the Act-1 graph, inserts RunState + the entry
    /// CurrentEncounter). Step at least once afterward so `load_encounter` generates the room + roster.
    /// The golden campaign never calls this, so it stays runless (byte-identical).
    pub fn start_run(&mut self, seed: u64) {
        crate::run::systems::transitions::start_run(
            self.app.world_mut(),
            seed,
            crate::run::systems::transitions::DEFAULT_RUN_HERO,
        );
    }

    /// Whether a run is active (RunState present).
    pub fn has_run(&self) -> bool {
        self.app.world().get_resource::<RunState>().is_some()
    }

    /// Emits a `StartRunRequest` (the death-screen R / character-select path). Step at least once
    /// afterward so `apply_start_run_request` runs the reset and `start_run` boots the fresh run.
    pub fn request_start_run(&mut self, hero_id: &str, seed: u64) {
        self.app.world_mut().send_event(
            crate::run::systems::reset::StartRunRequest { hero_id: hero_id.to_string(), seed },
        );
    }

    /// Total `AbilityInstance` entities in the world (they are separate top-level entities). Used to
    /// assert a clean teardown on restart (a dead player's / despawned enemy's instances are gone).
    pub fn ability_instance_count(&mut self) -> usize {
        let world = self.app.world_mut();
        let mut q = world.query::<&AbilityInstance>();
        q.iter(world).count()
    }

    /// Every `AbilityInstance` entity. Capture this before a restart, then assert none survive after
    /// (a run reset must despawn all of them — the fresh run spawns brand-new instance entities).
    pub fn ability_instance_entities(&mut self) -> Vec<Entity> {
        let world = self.app.world_mut();
        let mut q = world.query_filtered::<Entity, With<AbilityInstance>>();
        q.iter(world).collect()
    }

    /// Whether an entity is still alive (a despawned entity's handle never matches a reused index —
    /// generations differ — so this is a safe post-teardown liveness check).
    pub fn entity_exists(&self, entity: Entity) -> bool {
        self.app.world().get_entity(entity).is_ok()
    }

    /// The current `GameOverSummary`, if a run has ended (victory flag + captured hero/level).
    pub fn game_over_victory(&self) -> Option<bool> {
        self.app
            .world()
            .get_resource::<crate::game::state::GameOverSummary>()
            .map(|s| s.victory)
    }

    /// The current act (1–3), if a run is active.
    pub fn current_act(&self) -> Option<u8> {
        self.app.world().get_resource::<RunState>().map(|r| r.current_act)
    }

    /// The current graph node id, if a run is active.
    pub fn current_node(&self) -> Option<NodeId> {
        self.app.world().get_resource::<RunState>().map(|r| r.current_node)
    }

    /// The scaling depth of the live encounter (D5), if one is loaded.
    pub fn current_depth(&self) -> Option<u32> {
        self.app.world().get_resource::<CurrentEncounter>().map(|c| c.depth)
    }

    /// Nodes reachable in one step from the current node (the MapSelect branches).
    pub fn reachable_nodes(&self) -> Vec<NodeId> {
        self.app
            .world()
            .get_resource::<RunState>()
            .map(|r| r.act_graph.next_nodes(r.current_node))
            .unwrap_or_default()
    }

    /// `Debug` string of the live encounter type (e.g. `"Map { objective: KillAll }"`), if loaded.
    pub fn current_encounter_debug(&self) -> Option<String> {
        self.app
            .world()
            .get_resource::<CurrentEncounter>()
            .map(|c| format!("{:?}", c.encounter))
    }

    /// Whether the live encounter has finished spawning its roster.
    pub fn encounter_spawned(&self) -> bool {
        self.app
            .world()
            .get_resource::<CurrentEncounter>()
            .map(|c| c.spawned)
            .unwrap_or(false)
    }

    /// Overrides the live encounter with a synthetic node (test tool — bypasses the graph, keeps the
    /// existing RunState). Step once afterward so `load_encounter` builds the room + roster. Use for
    /// exercising a specific encounter type/objective/curse without seed-hunting the graph.
    pub fn set_current_encounter(
        &mut self,
        encounter: EncounterType,
        theme: Option<&str>,
        depth: u32,
        modifier: Option<&str>,
    ) {
        let node = EncounterNode {
            id: u32::MAX,
            column: depth as usize,
            encounter,
            theme: theme.map(|s| s.to_string()),
            modifier: modifier.map(|s| s.to_string()),
        };
        self.app
            .world_mut()
            .insert_resource(CurrentEncounter::for_node(&node, depth));
    }

    /// Drives the sim to `GameState::Menu` (the windowed boot state — the headless sim defaults to
    /// InRun, which never runs `enter_main_menu`). Steps once so the transition applies.
    pub fn enter_menu(&mut self) {
        self.app
            .world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Menu);
        self.step(1);
    }

    /// Picks hero `i` (0-based, `HeroDef::MANIFEST` order) on the character-select screen — presses
    /// the matching digit for one frame (drives `handle_character_select_input`). The sim must be in
    /// `GameState::CharacterSelect`.
    pub fn select_hero_index(&mut self, i: usize) {
        let key = match i {
            0 => KeyCode::Digit1,
            1 => KeyCode::Digit2,
            2 => KeyCode::Digit3,
            _ => KeyCode::Digit4,
        };
        self.tap_key(key);
    }

    /// Picks reachable branch `i` (0/1/2) in the MapSelect overlay — presses the matching digit for
    /// one frame (drives `handle_map_select`). The sim must be in `GameState::MapSelect`.
    pub fn pick_branch(&mut self, i: usize) {
        let key = match i {
            0 => KeyCode::Digit1,
            1 => KeyCode::Digit2,
            _ => KeyCode::Digit3,
        };
        self.tap_key(key);
    }

    /// Number of tagged `MapBoss` entities alive (for KillMapBoss / boss-room scenarios).
    pub fn map_boss_count(&mut self) -> usize {
        let world = self.app.world_mut();
        let mut q = world.query_filtered::<Entity, With<crate::enemy::components::MapBoss>>();
        q.iter(world).count()
    }

    /// Number of active ThroneRoom curse modifiers (0 outside a ThroneRoom).
    pub fn room_modifier_count(&self) -> usize {
        self.app
            .world()
            .get_resource::<RoomModifiers>()
            .map(|r| r.0.len())
            .unwrap_or(0)
    }

    /// All living `Enemy` entities (pack + bosses).
    pub fn enemy_entities(&mut self) -> Vec<Entity> {
        let world = self.app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Enemy>>();
        q.iter(world).collect()
    }

    /// All living tagged `MapBoss` entities.
    pub fn map_boss_entities(&mut self) -> Vec<Entity> {
        let world = self.app.world_mut();
        let mut q = world.query_filtered::<Entity, With<crate::enemy::components::MapBoss>>();
        q.iter(world).collect()
    }

    /// The `DamageDealtModifier` on an entity, if any (the Phase-5 depth-scaling damage multiplier).
    pub fn damage_dealt_modifier(&self, entity: Entity) -> Option<f32> {
        self.app.world().get::<DamageDealtModifier>(entity).map(|m| m.0)
    }

    /// Kills every living enemy this frame (lethal DamageEvent to each). Step afterward so
    /// `apply_damage` → `enemy_death` resolve and `check_objective` sees the cleared roster.
    pub fn kill_all_enemies(&mut self) {
        for e in self.enemy_entities() {
            self.deal_damage(e, 1.0e6);
        }
    }

    /// Directly sets the active ThroneRoom curse from a loaded RoomModifierDef id (test tool for the
    /// curse mechanism, independent of graph placement). Settle assets first.
    pub fn apply_room_curse(&mut self, id: &str) {
        let mods = {
            let world = self.app.world();
            let handle = world
                .resource::<RoomModifierLibrary>()
                .get(id)
                .unwrap_or_else(|| panic!("unknown room modifier '{id}'"))
                .clone();
            world
                .resource::<Assets<RoomModifierDef>>()
                .get(&handle)
                .unwrap_or_else(|| panic!("room modifier '{id}' not loaded yet — settle assets first"))
                .curse_modifiers
                .clone()
        };
        self.app.world_mut().resource_mut::<RoomModifiers>().0 = mods;
    }

    // ---------------------------------------------------------------- persistence / meta (Phase 8)

    /// Direct read access to the live `RunState`, if a run is active. All fields are `pub`, so
    /// scenarios can assert `unlocked_abilities`/`acquired_talents`/`elapsed_secs`/etc. directly.
    pub fn run_state(&self) -> Option<&RunState> {
        self.app.world().get_resource::<RunState>()
    }

    /// Direct read access to the always-present `MetaState` (unlocked heroes, run history, the
    /// saved in-progress run). All fields are `pub`.
    pub fn meta(&self) -> &crate::meta::state::MetaState {
        self.app.world().resource::<crate::meta::state::MetaState>()
    }

    /// Test-only: locks a hero (removes it from `MetaState.unlocked_heroes`) to exercise the
    /// locked-pick-refused path — no hero is actually locked by default (D3, all unlocked).
    pub fn lock_hero(&mut self, hero_id: &str) {
        self.app
            .world_mut()
            .resource_mut::<crate::meta::state::MetaState>()
            .unlocked_heroes
            .remove(hero_id);
    }

    /// Emits a `ResumeRunRequest` (the main-menu Resume input). Step at least once afterward so
    /// `apply_resume_request` runs the hydration.
    pub fn request_resume_run(&mut self) {
        self.app
            .world_mut()
            .send_event(crate::run::systems::persistence::ResumeRunRequest);
    }

    /// Drives the sim to `GameState::Login` (the windowed boot state ahead of Menu — the headless
    /// sim defaults to InRun, which never runs `enter_login`). Steps once so the transition applies.
    pub fn enter_login(&mut self) {
        self.app
            .world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Login);
        self.step(1);
    }

    /// A deterministic per-enemy signature (grid position + exact max-health bit pattern), sorted —
    /// there is no stable per-enemy-def id component, but position + health is enough to prove "the
    /// same roster spawned" across two independently-driven sims (the D1 resume-determinism test).
    pub fn enemy_roster_signature(&mut self) -> Vec<(i32, i32, u32)> {
        let world = self.app.world_mut();
        let mut q = world.query_filtered::<(&GridPosition, &Health), With<Enemy>>();
        let mut sig: Vec<(i32, i32, u32)> =
            q.iter(world).map(|(g, h)| (g.x, g.y, h.max.to_bits())).collect();
        sig.sort();
        sig
    }
}
