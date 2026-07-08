# Architecture Plan

_Written against the prototype at commit `af5126d` and `Mechanics.md` as of 2026-07-04._

---

## 1. Prototype Audit: Keep, Discard, Rewrite

### Keep without change

| Piece | File(s) | Why |
|---|---|---|
| `GridPosition` / `WorldPosition` / `Velocity` / `Facing` | `core/components.rs` | Clean separation of logical vs. render position. Axis-decomposed collision (slide-along-wall) is exactly right for tile-level density. Both player and enemies share this cleanly. |
| `FlowField` BFS resource + algorithm | `core/systems/flow_field.rs` | Correct for Vampire Survivors density. Radius cap (`FLOW_RADIUS`) is a practical optimisation to keep. |
| `CombatSet` ordering (Damage → Apply → Death) | `core/sets.rs` | Per-frame consistency is already modeled. Will grow a StatusEffect set but the chain pattern is right. |
| `DamageEvent` / `HealEvent` / `GainXpEvent` / `LevelUpEvent` | `core/events.rs` | Event-based indirection is exactly the right coupling model. `DamageEvent` needs one field added (see §4); everything else stays. |
| `apply_damage`, `apply_heal`, `apply_velocity`, `world_to_grid`, `sync_transform` | `core/systems/` | Single-responsibility leaf systems with no coupling to content. Keep verbatim. |
| `TileMap` sparse representation | `world/components.rs` | `HashSet<GridPosition>` for blocked tiles is cheap and correct. `is_blocked` / `in_bounds` can be kept as-is. |
| Mouse-to-world `Facing` update | `player/systems/update_player_facing.rs` | Camera-viewport unprojection. Unchanged. |
| `gain_experience` | `player/systems/experience.rs` | The mutation and overflow logic is right. The _consumer_ of `LevelUpEvent` changes completely (talent offer flow), not this emitter. |
| Enemy flow-field following | `enemy/systems/follow_flow_field.rs` | Works. |
| WASD movement | `player/systems/input.rs` | Keep as movement input only. |

### Discard

| Piece | Why |
|---|---|
| Hardcoded attack systems (`player_circle_attack`, `player_arc_attack`) | Attack execution becomes ability-system driven. Input slot → ability resolution is now indirect through the class definition. |
| Attack keybindings in `player_input` (Space, V) | Movement keys stay; attack bindings move into the input-slot → ability layer. |
| `spawn_enemy_over_time` global timer | Replaced by room-specific encounter spawning driven by the act graph. |
| `archetypes()` Rust function | Enemy type definitions move to RON assets. The _pattern_ (archetype struct copied onto entity components at spawn) is correct; the _location_ (compiled Rust) is not. |
| `generate_map` blob algorithm (as the production algorithm) | Replaced by themed room + act graph generation. It can survive as a fallback room interior generator if rooms need interior obstacle scatter, but it is not the primary world structure. |
| `player_death` as bare despawn + log | Replaced by a `GameState::GameOver` transition. |
| `apply_level_up_reward` log stub | Replaced by the leveling/talent-offer flow. |

### Rewrite structurally (same role, different model)

| Piece | Change |
|---|---|
| `PlayerPlugin` / player components | Add class identity, stance state, unlocked abilities, talent list. Input now resolves through stance → slot → ability. |
| `EnemyPlugin` | Enemies get ability kits and an AI behavior ID, mirroring the player ability structure. |
| `WorldPlugin` | Grows from one flat room to act graph + per-node room generation + encounter management. |
| `GamePlugin` | Add `GameState` enum with per-state system configuration (menu, character select, in-run, paused, game-over). |
| `projectile::Projectile` | Currently VFX-only. Needs real movement + collision system. Keep the component names; add the missing systems. |

---

## 2. Plugin / Module Breakdown

```
src/
  main.rs
  constants.rs          ← global numeric tuning only (keep)
  game/
    plugin.rs           ← registers GameState, orchestrates all other plugins
  core/
    plugin.rs           ← movement, CombatSet, events, status effects
    components.rs       ← GridPos, WorldPos, Velocity, Facing, Health, LastHitBy
    events.rs           ← DamageEvent (extended), HealEvent, GainXpEvent, LevelUpEvent
    sets.rs             ← CombatSet + StatusSet
    systems/            ← apply_damage, apply_heal, apply_velocity, flow_field, …
  ability/
    plugin.rs           ← registers BehaviorRegistry, drives ability execution
    assets.rs           ← AbilityDef Bevy asset (RON loader)
    components.rs       ← AbilityInstance, CooldownTimer, AbilityInputSlot
    behavior.rs         ← BehaviorRegistry resource, AbilityHook trait
    systems/            ← execute_ability (per behavior type), resolve_params
  talent/
    plugin.rs           ← registers TalentDef assets, offer generator, hook installers
    assets.rs           ← TalentDef Bevy asset
    components.rs       ← AcquiredTalents, ActiveHooks
    modifier.rs         ← StatModifier, ModOp, stat resolution
    offer.rs            ← TalentOfferState, offer generation logic
    systems/            ← apply_talent_modifiers, handle_merchant_ops
  status/
    plugin.rs
    assets.rs           ← StatusEffectDef (RON)
    components.rs       ← ActiveStatusEffects, per-effect timer components
    systems/            ← tick_status_effects, apply_cross_interactions
  hero/
    plugin.rs
    assets.rs           ← HeroDef RON asset
    components.rs       ← HeroId, ActiveStance, ClassResource
    systems/            ← resolve_input_slot, handle_stance_swap, update_stance_abilities
  zone/
    plugin.rs
    components.rs       ← PersistentZone, ZoneAnchor
    systems/            ← zone_lifetime, build_player_zone_presence, move_anchored_zones
  enemy/
    plugin.rs
    assets.rs           ← EnemyDef, ThemeDef RON assets
    components.rs       ← Enemy, EnemyRarity, AiBehaviorId, MoveSpeed, AttackStats, XpReward, AttackCooldown
    behavior.rs         ← AiBehaviorRegistry, EnemyAiHook trait
    systems/            ← follow_flow_field, enemy_attack, enemy_death, execute_enemy_abilities
  world/
    plugin.rs
    components.rs       ← TileMap (keep), RoomLayout
    graph.rs            ← ActGraph, EncounterNode, ObjectiveType
    generator.rs        ← per-room tile generation (can reuse blob logic as interior scatter)
    systems/            ← build_act_graph, load_room, spawn_encounter, render_map
  run/
    plugin.rs
    state.rs            ← RunState resource (seed, graph position, hero state, levels, talents)
    rng.rs              ← RunRng resource
    systems/            ← save_run_state, encounter_complete, transition_acts
  progression/
    plugin.rs
    systems/            ← consume_level_up, unlock_ability, generate_talent_offer, handle_offer_choice
  meta/
    plugin.rs           ← deliberately decoupled from `run`
    state.rs            ← MetaState resource (unlocked heroes, scoreboard, in-progress save slot)
    persistence.rs      ← serialize/deserialize MetaState and RunState
  pickup/
    plugin.rs           ← keep; extend PickUpKind
    components.rs, systems/  ← keep
  projectile/
    plugin.rs
    components.rs       ← Projectile, Lifetime, Velocity, PierceCount
    systems/            ← move_projectiles, projectile_enemy_collision
  camera/               ← keep as-is
  ui/
    plugin.rs           ← registers UI plugin, reads from run/talent/hero data; no data ownership
    screens/            ← talent_offer, map_graph, character_select, hud, merchant
```

**Responsibilities per domain:**

- `core` — physics, combat event bus, health, status effect infrastructure. No content.
- `ability` — execution of all ability shapes. Does not know what class owns an ability.
- `talent` — offer generation, uniqueness enforcement, modifier stack. Does not know ability internals.
- `status` — apply/tick/remove status effects and cross-effect interactions. Does not know who applied them.
- `hero` — class identity and stance. Translates "player pressed left-click in stance X" → `AbilityId`. Does not execute abilities.
- `zone` — lifecycle and spatial query for persistent ground zones. Does not know zone semantics.
- `enemy` — enemy lifecycle, AI, and their own ability execution.
- `world` — tile map + act graph construction + encounter lifecycle.
- `run` — authoritative run state and seeded RNG. Single source of truth for resumability.
- `progression` — level-up flow. Reads `run::RunState`, emits ability unlock and talent offer events.
- `meta` — account-level state, completely decoupled from `run`.
- `ui` — reads data from all other domains, owns nothing.

---

## 3. Core Data Structures and Interfaces

All subsystems are detailed below. Each component, resource, or trait is justified against a specific requirement.

---

### 3.1 DamageEvent Extension

_Requirement: fire/frost cross-interactions require knowing the damage's element at the application site._

```
DamageEvent {
    target:   Entity,
    amount:   f32,
    source:   Entity,
    tags:     SmallVec<[DamageTag; 2]>,   // usually 0–2 tags
}

enum DamageTag { Physical, Fire, Frost, Holy, Shadow, Arcane }
```

The `tags` field is the only change to the existing `DamageEvent`. `apply_damage` does not use tags; `status::apply_cross_interactions` listens to DamageEvents and uses tags to trigger frostbite/blaze removal rules. All existing callers just pass an empty slice.

---

### 3.2 Hero / Class Definition

_Requirement: each class defines its starting abilities, the two unlocked-ability pools, its class-wide passive talent list, whether it has a stance, and any special resource model._

`HeroDef` is a Bevy asset loaded from RON. One file per class.

```ron
// assets/heroes/blood_death_knight.ron
(
    id: "blood_death_knight",
    display_name: "Blood Death Knight",
    base_stats: (
        max_health: 200.0,
        move_speed: 35.0,
    ),
    resource_model: HealthBased,       // health IS the leech resource; no secondary bar
    has_stance: false,
    level_1_abilities: ["death_strike", "dnd", "companion"],
    band_2_3_pool: ["blood_boil", "heart_strike"],   // pick one at L2, one at L3
    band_4_6_pool: ["abomination_limb", "purgatory", "amz"],
    class_passive_pool: [
        "bdk_passive_no_heal_above_35",
        "bdk_passive_low_health_damage",
        "bdk_passive_overkill_leech",
        "bdk_passive_health_and_healing",
        "bdk_passive_blood_boil_spawns_dnd",
    ],
)
```

**Components placed on the player entity:**

```
HeroId(pub String)                     // which HeroDef is active
ActiveStance(pub StanceId)             // always present; "default" for no-stance heroes
```

**`InputSlot` enum** — the four bindable actions:

```
enum InputSlot { Basic, Special, Movement, StanceSwap }
```

**Stance resolution**: `HeroDef` contains `stance_ability_map: HashMap<StanceId, HashMap<InputSlot, AbilityId>>`. When the player presses Basic, the `hero` system reads `ActiveStance`, looks up the slot, and emits `TriggerAbility { ability_id }`. For heroes without a stance, a single `"default"` stance covers all slots.

---

### 3.3 Ability System

_Requirement: covers melee cones, projectiles, periodic self-centered zones, dropped zones, orbiting effects, leap/dash, channel-while-moving, and summons that replay another ability. Input → ability resolution must be indirect._

#### AbilityDef (RON asset)

```ron
// assets/abilities/death_strike.ron
(
    id: "death_strike",
    display_name: "Death Strike",
    unlock_schedule: Level1,
    behavior: "melee_cone",            // key into BehaviorRegistry
    base_params: {
        "damage":      10.0,
        "range":       60.0,
        "half_angle":   0.785,         // pi/4
        "cooldown":     1.2,
        "leech_percent": 5.0,
    },
    talent_pool: [
        "death_strike_leech_common_1",
        "death_strike_leech_common_2",
        "death_strike_range_common",
        "death_strike_bone_shield_epic",
    ],
)
```

`unlock_schedule` is one of `Level1 | Band(u8, u8)`.

**`base_params`** is a `HashMap<StatId, f32>`. Stats are strings so new stats can be added by content without code changes. Systems read params through `resolve_params`, which applies the modifier stack.

#### AbilityInstance (component, child entity of player)

```
AbilityInstance {
    def_id:  AbilityId,
    cooldown: Timer,
    stance:  Option<StanceId>,    // None = available in all stances
}
```

Each unlocked ability is a separate entity, parented to the player, carrying `AbilityInstance`. The ability execution system queries `AbilityInstance` children; the stance filter narrows which abilities are eligible.

#### BehaviorRegistry (resource, registered at plugin build)

```
trait AbilityHook: Send + Sync {
    // `params` is the modifier-resolved param map for this ability this frame.
    // `ctx` provides entity, position, facing, event writers — no &mut World access.
    fn execute(&self, ctx: &AbilityContext, params: &ResolvedParams);
}

struct BehaviorRegistry {
    behaviors: HashMap<BehaviorId, Box<dyn AbilityHook>>,
}
```

Built-in behavior IDs registered at startup: `"melee_cone"`, `"projectile"`, `"periodic_self_zone"`, `"dropped_zone"`, `"orbiting"`, `"leap_to_target"`, `"channel_while_moving"`, `"summon"`. Adding a new shape = implement `AbilityHook` + one `registry.register(id, hook)` call in a plugin.

The ability execution system, per frame, for each ready `AbilityInstance`:
1. Calls `resolve_params(ability_id, &acquired_talents)` → `ResolvedParams`
2. Looks up `behavior_id` in `BehaviorRegistry`
3. Builds `AbilityContext` from the entity's position/facing/event writers
4. Calls `hook.execute(&ctx, &params)` — emits `DamageEvent`, `SpawnProjectileEvent`, etc.

**Behavior hooks from talents** (e.g., bone shield, Flamestrike's epic): these are installed as pre/post hooks alongside the base behavior, discussed in §3.4.

---

### 3.4 Talent System

_Requirement: 100+ talent variants, numeric modifiers (common/most rare), behavior rewrites (some rare/epic), uniqueness as stack-cap or mutual-exclusion. New talent = data file + optional one hook._

#### TalentDef (RON asset)

```ron
// assets/talents/death_strike_bone_shield_epic.ron
(
    id: "death_strike_bone_shield_epic",
    display_name: "Bone Shield",
    ability_scope: Some("death_strike"),   // None = class-wide or general
    rarity: Epic,
    uniqueness: Exclusive,                 // only one copy allowed
    effect: Behavior("bone_shield_on_kill"),   // registered code hook
)

// assets/talents/death_strike_leech_common_1.ron
(
    id: "death_strike_leech_common_1",
    display_name: "Improved Leech",
    ability_scope: Some("death_strike"),
    rarity: Common,
    uniqueness: Stack(3),
    effect: Modifier((
        stat: "leech_percent",
        op: MultiplyAdd(0.20),
    )),
)
```

```
enum TalentEffect {
    Modifier(StatModifier),
    Behavior(HookId),
}

struct StatModifier {
    stat:          StatId,
    op:            ModOp,
    ability_scope: Option<AbilityId>,   // None = applies globally
}

enum ModOp {
    Add(f32),
    MultiplyAdd(f32),   // multiplicative bonus on the base value
    Override(f32),      // replaces entirely (epic-level only; use sparingly)
}
```

```
enum UniquenessConstraint {
    None,
    Stack(u8),                       // unique[3]
    Exclusive,                       // one copy
    MutuallyExcludes(TalentId),      // unique[Fiery Ent / Earth Ent]
}
```

#### Modifier resolution

`resolve_params(ability_id, talents) → ResolvedParams` is a pure function (no ECS):

1. Start with `base_params` from `AbilityDef`.
2. For each `TalentEffect::Modifier` in the player's talent list whose `ability_scope` matches `ability_id` or is `None`:
   - Accumulate all `Add` bonuses into an additive pool per stat.
   - Accumulate all `MultiplyAdd` bonuses into a multiplicative pool per stat.
3. Final value = `(base + additive_sum) * (1.0 + multiplicative_sum)`. Overrides are applied last.

This is deterministic, data-driven, and requires no match statement to add a new stat.

#### Behavior hooks from talents

When a `Behavior(hook_id)` talent is acquired, the talent plugin inserts a marker component on the player entity:

```
struct ActiveHook(pub HookId);    // one component per active behavior hook talent
```

The ability execution system checks: before calling the base behavior, collect all `ActiveHook` components on the player that belong to the current ability's pre-hook list. After calling the base behavior, collect post-hook list. Both lists come from metadata in `AbilityDef` (a `hooks: Vec<(HookPhase, HookId)>` field, pre-populated by content authors). Each `HookId` maps to an `AbilityHook` in a separate `HookRegistry`. The hook is only executed if the player has the corresponding `ActiveHook` component, so un-acquired talents have zero runtime cost.

When a talent is removed (merchant), the corresponding `ActiveHook` component is removed.

#### AcquiredTalents component

```
struct AcquiredTalents {
    // (talent_id, count): count > 1 for stack[N] talents
    entries: Vec<(TalentId, u8)>,
}
```

Uniqueness checks for offer generation:
- `Stack(n)`: only offer if `count < n`
- `Exclusive`: only offer if `count == 0`
- `MutuallyExcludes(other)`: only offer if `other` not present in entries

---

### 3.5 Status Effect System

_Requirement: bleed, blaze, frostbite, holy mark, root, stun; each with its own stacking rule; cross-effect interactions (fire→removes frostbite, frost→removes blaze). New element should not touch existing ones._

#### StatusEffectDef (RON asset)

```ron
// assets/status_effects/frostbite.ron
(
    id: "frostbite",
    display_name: "Frostbite",
    stacking: RefreshOnReapply,
    base_duration: 4.0,
    on_apply_hooks: [],
    on_tick_hooks:  [],
    on_remove_hooks: [],
    // Applied by fire damage tags; removes itself, removes nothing
    removed_by_tags: [Fire],
    // Applying frostbite does not itself remove blaze
    removes_on_apply: [],
)

// assets/status_effects/blaze.ron
(
    id: "blaze",
    stacking: RefreshOnReapply,
    base_duration: 4.0,
    removed_by_tags: [Frost],
    removes_on_apply: [],
)
```

Cross-interactions are fully encoded in the definition files. The status system listens to `DamageEvent`, checks `event.tags` against each active status's `removed_by_tags` on the target, and despawns matching effect entities. Adding a new element that cancels an existing one means editing only the new element's `removes_on_apply` list — no existing file changes.

#### Per-entity status effect representation

Each active status instance is a child entity of the target:

```
struct StatusEffectInstance {
    def_id:   StatusEffectId,
    owner:    Entity,           // the entity that applied it (for kill credit, damage attribution)
    duration: Timer,
    stacks:   u8,
}
```

Multiple stacks = multiple entities (for `StackCapped(n)` effects like bleed). For `RefreshOnReapply`, there is always at most one instance; re-application resets the timer.

Querying "does target have frostbite": `Query<&StatusEffectInstance, With<Parent>>` filtered by `def_id == "frostbite"`. For the common "is the player standing in zone X" pattern, a pre-built `PlayerZonePresence` resource is used instead (§3.6).

#### StatusSet (added to CombatSet chain)

```
CombatSet::Damage → CombatSet::Apply → StatusSet::Tick → StatusSet::CrossInteract → CombatSet::Death
```

`StatusSet::Tick` applies per-tick damage (bleed, blaze). `StatusSet::CrossInteract` processes tag-based removals from DamageEvents that frame.

---

### 3.6 Persistent Zones

_Requirement: D&D, Consecrated Ground, Tree Conduit are persistent zones queryable by other systems. "Is the player inside zone X" recurs across classes._

```
struct PersistentZone {
    zone_type: ZoneTypeId,
    owner:     Entity,
    radius:    f32,
    duration:  Timer,
    anchor:    ZoneAnchor,
}

enum ZoneAnchor {
    Fixed(Vec2),
    Follow(Entity),    // AMZ epic talent: zone follows player
}
```

Each zone is a world entity. Every frame, `build_player_zone_presence` sweeps all `PersistentZone` entities, tests player distance against radius, and writes the result to:

```
struct PlayerZonePresence {
    active_zone_types: HashSet<ZoneTypeId>,
}
```

Any system that gates on zone presence reads this resource. No system directly queries zone entities for presence tests; `PlayerZonePresence` is the spatial cache. Zone entities are queried only for lifetime management and the `Follow` anchor update.

Zone types are strings (registered via RON in `AbilityDef`), so a new zone type is just a new name — no code change.

---

### 3.7 Leveling & Talent Offer Flow

_Requirement: fixed L1 abilities, band-2/3 pool (pick 1/level, without replacement), band-4/6 pool (same), then talent-choice offers (1 of 3, decline allowed), rare/epic-only for special events, merchant ops._

#### LevelUpPhase resource

```
struct LevelUpFlowState {
    phase:                LevelUpPhase,
    band_2_3_remaining:   Vec<AbilityId>,   // shuffled at run start with RunRng
    band_4_6_remaining:   Vec<AbilityId>,
    pending_offer:        Option<TalentOffer>,
}

enum LevelUpPhase {
    AbilityUnlock,      // L2–L6: still drawing from ability pools
    TalentChoices,      // all core abilities unlocked; subsequent levels offer talents
}

struct TalentOffer {
    options:    [Option<TalentId>; 3],   // Some = offered, None = pool exhausted
    rarity_filter: Option<RarityFilter>, // None = any; Some(RareOrAbove) for special events
}
```

**Flow on `LevelUpEvent`:**

1. If `phase == AbilityUnlock` and the appropriate band pool is non-empty: pop one ability ID, emit `UnlockAbility { id }`, check if all bands are now empty → if so, transition `phase = TalentChoices`.
2. If `phase == TalentChoices`: call `generate_offer(rng, acquired_talents, all_talent_defs) → TalentOffer`, store in `pending_offer`, push `GameState::TalentPicker`.

**`generate_offer`** samples from the eligible talent pool: all talents from the player's unlocked abilities + class passives + general passives, filtered by uniqueness constraints and (for special events) rarity filter. Uses `RunRng` exclusively.

**Merchant operations:**
- Remove: pop one `(TalentId, u8)` from `AcquiredTalents`, remove corresponding `ActiveHook` if present, rebuild modifier cache.
- 3-for-1 trade: remove three specified talents, call `generate_offer` with `RarityFilter::HigherThanHighest(removed_rarities)`.

---

### 3.8 Run Graph & Room System

_Requirement: seeded, non-repeating, branching 3×3×15 graph; typed nodes; objective type as map node property; 5 pluggable per-theme rosters._

#### Act graph

```
struct ActGraph {
    nodes: HashMap<NodeId, EncounterNode>,
    edges: Vec<(NodeId, NodeId)>,
    entry: NodeId,
}

struct EncounterNode {
    id:           NodeId,
    encounter:    EncounterType,
    theme:        Option<ThemeId>,   // None for Merchant, ActBoss
    modifier:     Option<ModifierId>, // kiss/curse (see §6, Open Question 1)
}

enum EncounterType {
    Map       { objective: ObjectiveType },
    BossRoom,
    ActBoss,
    ThroneRoom,
    Merchant,
}

enum ObjectiveType {
    Survive { duration_secs: f32 },
    KillAll,
    KillMapBoss { boss_id: EnemyId },
}
```

**Graph generation**: `build_act_graph(rng: &mut RunRng) → ActGraph` uses only `RunRng` so that map layout is seed-deterministic regardless of what other systems have consumed from `thread_rng`.

#### ThemeDef (RON asset)

```ron
// assets/themes/sand_dune.ron
(
    id: "sand_dune",
    display_name: "Sand Dune",
    common_enemy_pool: ["scorpion", "vulture", "tusken", "flame_demon", "oil_elemental"],
    boss_pool:         ["king_scorpion", "tusken_lord", "flame_cultist", "undead_lord"],
    map_boss_pool:     ["king_scorpion", "tusken_lord"],
    ambient_tint:      (1.0, 0.9, 0.7, 1.0),
)
```

Adding a new theme = one RON file referencing already-defined `EnemyId`s. No code changes.

---

### 3.9 Enemy / AI Framework

_Requirement: no enemy abilities defined yet; design the interface they'll plug into, structurally similar to the player ability system._

#### EnemyDef (RON asset)

```ron
// assets/enemies/scorpion.ron
(
    id: "scorpion",
    display_name: "Scorpion",
    rarity: Common,
    base_stats: (
        max_health: 40.0,
        move_speed: 18.0,
        size_radius: 14.0,
    ),
    ai_behavior: "melee_chaser",    // key into AiBehaviorRegistry
    abilities: [
        (
            behavior: "contact_melee",
            params: { "damage": 8.0, "range": 30.0, "cooldown": 1.0 },
        ),
    ],
    xp_value: 5,
    drop_table: "common_enemy",
)
```

The `abilities` list is structurally identical to the player ability system: behavior ID → params → execution via `BehaviorRegistry`. Enemy abilities are simpler (no stance, no talent modification) but reuse the same execution path.

#### AiBehaviorRegistry

```
trait EnemyAiHook: Send + Sync {
    fn update(&self, ctx: &mut EnemyAiContext);
}

struct AiBehaviorRegistry {
    behaviors: HashMap<AiBehaviorId, Box<dyn EnemyAiHook>>,
}
```

Built-in: `"melee_chaser"` (current flow-field follower), `"ranged_caster"` (stop at range + cast), `"stationary"`, `"boss"` (multi-phase, TBD). New enemy AI = implement `EnemyAiHook` + register.

**Enemy rarity tiers** map to pack/boss roles in room generation:

```
enum EnemyRarity { Common, Elite, MapBoss, ActBoss }
```

Room spawner pulls from `ThemeDef::common_enemy_pool` for pack enemies, and from `boss_pool` / `map_boss_pool` for the room boss node.

---

### 3.10 Persistence: Two Decoupled Scopes

_Requirement: resumable per-run state, account-level meta state. These must not entangle._

#### RunState (resource, lives only during an active run)

```
struct RunState {
    seed:               u64,
    hero_id:            HeroId,
    act_graph:          ActGraph,
    current_node:       NodeId,
    player_health:      f32,
    player_level:       u32,
    unlocked_abilities: Vec<AbilityId>,
    acquired_talents:   Vec<(TalentId, u8)>,
    level_flow:         LevelUpFlowState,
}
```

Serialized to disk at every node transition. Deserialized on "Resume Run". No reference to `MetaState`.

#### MetaState (resource, lives for the entire session)

```
struct MetaState {
    unlocked_heroes:    HashSet<HeroId>,
    run_history:        Vec<RunRecord>,
    in_progress_run:    Option<SavedRunState>,
}

struct RunRecord {
    hero_id:   HeroId,
    act_reached: u8,
    score:     u32,
    timestamp: u64,
}
```

`SavedRunState` is a serialized `RunState`. On "Start New Run", `MetaState::in_progress_run` is cleared and a fresh `RunState` is created. On "Resume Run", `RunState` is hydrated from the saved blob.

The `meta` plugin is inserted unconditionally at app startup and persists across `GameState` transitions. The `run` plugin resource is inserted only when a run begins and removed (or replaced) on game-over or return to menu.

---

### 3.11 Seeded RNG

_Requirement: one run seed must deterministically drive map generation, talent offers, and pack composition without being perturbed by unrelated randomness consumers._

```
struct RunRng(SmallRng);   // rand::rngs::SmallRng seeded from RunState.seed
```

Rule: any system that must be run-seed-deterministic takes `ResMut<RunRng>`. All other systems (VFX, particle angles, audio variation) use `rand::thread_rng()`. The `RunRng` is initialized exactly once per run from the seed, never reset mid-run. `RunRng` is part of `RunState` for serialization (save the RNG state, not just the seed, so a resumed run continues from where it left off).

Consuming from `thread_rng()` in a VFX system cannot perturb `RunRng` because they are separate streams.

---

## 4. Worked Data-Schema Example

_Death Strike (Blood DK basic attack) + three of its talents: one plain numeric, one behavior-rewriting epic, and one that uses the zone-presence system._

### Ability definition

```ron
// assets/abilities/death_strike.ron
(
    id: "death_strike",
    display_name: "Death Strike",
    unlock_schedule: Level1,
    behavior: "melee_cone",
    hooks: [
        // pre-hook: bone shield checks kill count before base behavior fires
        (phase: Post, hook_id: "bone_shield_on_kill"),
    ],
    base_params: {
        "damage":        10.0,
        "range":         60.0,
        "half_angle":     0.785,
        "cooldown":       1.2,
        "leech_percent":  5.0,
        // bone shield kill threshold; only meaningful if hook is active
        "bone_shield_kill_threshold": 5.0,
    },
    talent_pool: [
        "death_strike_leech_common",
        "death_strike_range_common",
        "death_strike_bone_shield_epic",
    ],
)
```

### Talent 1 — Plain numeric (common)

```ron
// assets/talents/death_strike_leech_common.ron
(
    id: "death_strike_leech_common",
    display_name: "Improved Leech",
    ability_scope: Some("death_strike"),
    rarity: Common,
    uniqueness: Stack(3),     // can be taken up to 3 times
    effect: Modifier((
        stat: "leech_percent",
        op: MultiplyAdd(0.20),    // +20% leech per copy; stacks multiplicatively
    )),
)
```

**How a content author adds this**: Write the RON file. Reference its ID in `death_strike.ron`'s `talent_pool`. Done. `resolve_params` picks it up automatically; no code changes.

### Talent 2 — Behavior-rewriting epic

```ron
// assets/talents/death_strike_bone_shield_epic.ron
(
    id: "death_strike_bone_shield_epic",
    display_name: "Bone Shield",
    ability_scope: Some("death_strike"),
    rarity: Epic,
    uniqueness: Exclusive,
    effect: Behavior("bone_shield_on_kill"),
)
```

**The hook** (one small Rust struct, registered once in `AbilityPlugin::build`):

```
// ability/hooks/bone_shield.rs
struct BoneShieldOnKill;

impl AbilityHook for BoneShieldOnKill {
    fn execute(&self, ctx: &AbilityContext, params: &ResolvedParams) {
        let threshold = params.get("bone_shield_kill_threshold") as u32;
        // ctx provides access to per-ability state via a typed component on the ability entity
        // Increments a kill counter; when threshold reached, emits GainShieldEvent
    }
}

// In AbilityPlugin::build:
registry.register("bone_shield_on_kill", BoneShieldOnKill);
```

The hook is registered once. The talent is data. The ability's `hooks` list in the RON tells the execution system to call this hook if the player has the corresponding `ActiveHook` component (installed when the talent is acquired). If the talent is not taken, the hook call never happens — zero branch cost at runtime.

### Talent 3 — Zone-interaction (rare, unique)

```ron
// assets/talents/blood_boil_dnd_range_rare.ron
(
    id: "blood_boil_dnd_range_rare",
    display_name: "Empowered Reach",
    ability_scope: Some("blood_boil"),
    rarity: Rare,
    uniqueness: Exclusive,
    // Pure modifier: doubles range, but only effective inside D&D.
    // The zone condition is encoded in the hook, not the modifier.
    effect: Behavior("blood_boil_dnd_range"),
)
```

The hook reads `Res<PlayerZonePresence>` and conditionally doubles the `range` param before the ability executes. No D&D code is modified; no Blood Boil base code is modified. The hook is ~5 lines.

---

## 5. Scalability Check

_The test: does adding each of the following require touching unrelated code or core match statements?_

**New class (e.g., a fifth hero)**
- Add `assets/heroes/new_hero.ron`
- Add ability RON files for its abilities
- Add talent RON files for its talents
- If a new input mechanic: add one `InputSlot` variant (enum, not a match in logic)
- If a new resource model: add one `ResourceModel` variant + one UI bar
- Core systems untouched. ✓

**New talent on an existing ability**
- Write `assets/talents/new_talent.ron`
- Reference it in the ability's `talent_pool` list
- If numeric: `effect: Modifier(...)` — no code at all
- If behavior-rewriting: implement one `AbilityHook` struct + `registry.register(...)` — one file, one registration line
- Offer generator picks it up automatically via the talent asset loader. ✓

**New enemy**
- Write `assets/enemies/new_enemy.ron`
- Reference its `id` in `ThemeDef::common_enemy_pool`
- If a new AI behavior: implement `EnemyAiHook` + register — same pattern as ability hooks
- No existing enemy file or system touched. ✓

**New map theme**
- Write `assets/themes/new_theme.ron` referencing existing enemy IDs
- Add its `ThemeId` to the world generator's theme pool (one entry in a config RON)
- No code changes. ✓

All four cases pass. The one exception is genuinely novel _mechanic_ territory (a new resource model, a new AI behavior archetype) which requires a small code hook — but that is exactly the design intent.

---

## 6. Open Questions — Resolved 2026-07-04

**1. Throne Room = kiss/curse room? ✓ Resolved**

ThroneRoom IS the kiss/curse room. On entering a ThroneRoom node:
- The player receives a **significant reward**: pick 1 of 3 rare (or better) talents before the fight.
- The room applies a **mandatory curse modifier** for the duration (e.g. no regen, enemies deal double damage, player is slowed X%). Modifier comes from a `RoomModifierDef` RON asset assigned at graph-generation time.
- The **map layout** is a distinct "throne room" geometry (not drawn from the normal room pool). The room generator branches on `EncounterType::ThroneRoom` and calls a dedicated layout function.

`EncounterNode` carries `modifier: Option<ModifierId>`. For ThroneRoom nodes this is always `Some(...)`. For regular Map nodes it is always `None`. No other node type carries a modifier.

**2. Meta-progression between runs ✓ Resolved**

No. Only hero unlocks and the scoreboard persist. `MetaState` stays thin — no currency, no permanent stat trees, no upgrade economy. Power fully resets each run.

**3. "Log In" scope ✓ Resolved**

**Local only** for now. "Log In" in the user-flow sketch is a local profile screen. `MetaState` serializes to a local file (RON via serde). A future WASM/web demo is planned but out of scope; the persistence format should use serde so the backend can be swapped later without touching the data structures.

**4. Q binding for Death Knight and Paladin ✓ Resolved**

DK and Paladin have **no stance**. Q is intentionally unbound for them. The `HeroDef` for these classes sets `has_stance: false`; the hero system maps `InputSlot::StanceSwap` to a no-op. No Q ability slot is reserved for future use — if one is added later it will be designed then.

**5. Scope ✓ Resolved**

Generate scaffold stub files as the next step. Each stub documents what it should contain and how it interacts with other modules. See §8 (Migration Order) for the phase each stub belongs to.

---

## 7. Migration Order

A suggested sequence that keeps the game playable at each step and avoids large-bang rewrites.

**Phase 0 — Foundation (no visible change)**
1. Add `GameState` enum to `GamePlugin`. Gate existing systems to `InState(GameState::InRun)`.
2. Add `DamageTag` to `DamageEvent` (empty slice by default; backward-compatible).
3. Add `RunRng` resource. Replace `rand::thread_rng()` in `generate_map` with `RunRng`.
4. Create `docs/`, `assets/abilities/`, `assets/talents/`, `assets/heroes/`, `assets/enemies/`, `assets/themes/` directories.

**Phase 1 — Ability system (replaces hardcoded attacks)**
1. Implement `BehaviorRegistry` + `AbilityHook` trait.
2. Implement `melee_cone` and `projectile` built-in behaviors (maps to existing `player_circle_attack` / `player_arc_attack` logic).
3. Write `AbilityDef` RON loader. Write `death_strike.ron` and `dnd.ron` with placeholder params.
4. Add `AbilityInstance` child-entity spawning to `PlayerPlugin::spawn_player`.
5. Wire input slot → ability resolution through the stub `HeroDef` (single stance, no RON yet).
6. Delete `player_circle_attack` / `player_arc_attack` once the ability system reproduces their behavior.

**Phase 2 — Talent system**
1. Implement `TalentDef` RON loader, `AcquiredTalents` component, `resolve_params`.
2. Implement `LevelUpFlowState` + progression plugin consuming `LevelUpEvent`.
3. Wire talent offer state to a minimal UI (list 3 options, press 1/2/3 to pick).
4. Add 2–3 numeric talents for Death Strike to validate the stack.

**Phase 3 — Status effects**
1. Implement `StatusEffectDef` loader, per-entity status instance entities, tick system.
2. Implement cross-interaction removal via `DamageTag`.
3. Add bleed (Druid) and blaze/frostbite (Mage) definitions.

**Phase 4 — Stance system + second class** _(complete 2026-07-05 — focused vertical slice; see §8.6, docs/phase4-plan.md)_
1. ✅ Implemented `HeroDef` RON loader (via the generic `DefLibrary<T>`), `HeroIdentity` +
   `ActiveStance` components, and input-slot → ability resolution (`HeroPlugin`).
2. ✅ Added **Mage** (chosen per §8.2 — least extra machinery) with Fire/Ice stances + Q swap;
   Death Knight formalized as the default `HeroDef`. Heavier Mage subsystems deferred (§8.6).

**Phase 5 — Enemy ability system + AI registry** _(complete 2026-07-05 — full scope incl. a ranged
caster + faction-aware engine + data-only scaling; see §8.7, docs/phase5-plan.md)_
1. ✅ `EnemyDef` RON loader (via `DefLibrary<T>`); the 3 archetypes ported to `.enemy.ron`;
   `enemy/archetypes.rs` deleted.
2. ✅ AI dispatch — an **`AiBehavior` component enum** (not the scaffold's trait registry, which
   couldn't express world-accessing movement AI); the flow-field follower is gated to `MeleeChaser`.
3. ✅ Contact melee as an auto-cast enemy `AbilityDef` (`contact_melee` behavior), through the same
   faction-aware execute path; the hardcoded `enemy_attack` is deleted. Plus (full scope): a ranged
   caster (`spitter`) with enemy projectiles hitting the player, a data-only enemy-scaling model, and
   the `suppress_abilities` wiring.

**Phase 6 — Zone system** _(complete 2026-07-05 — full scope incl. occupant DoT/regen + AMZ
projectile blocking + the code-driven ability-hook system; see §8.8, docs/phase6-plan.md)_
1. ✅ `PersistentZone` / `ZoneAnchor` / `PlayerZonePresence` wired live (the scaffold `zone` module
   joined `lib.rs`/`GameLogicPlugin`).
2. ✅ D&D + Tree Conduit as zone emitters (a new `AbilityDef.zone: Option<ZoneSpec>` + `dropped_zone`
   behavior); plus Consecrated Ground (DoT) + AMZ (blocking) demonstrators.
3. ✅ First zone-conditioned talent (Blood Boil ×2 range inside D&D) — implemented as the **first
   code-driven ability hook** (`HookRegistry` + `AbilityHook`), which paid the §8.5
   `execute_ready_abilities` resolve/apply-split debt.

**Phase 7 — Act graph + room system** _(complete 2026-07-05 — full scope incl. ThroneRoom + Merchant;
the golden master stayed byte-identical; see §8.9, docs/phase7-plan.md)_
1. ✅ `ActGraph` / `EncounterNode` + a **pure, seeded** `build_act_graph` (Slay-the-Spire columns;
   `world/graph.rs` compiled).
2. ✅ `ThemeDef` loader (`.theme.ron` `DefAsset`) + all 5 themes (D4: pools → existing enemies + a
   `warlord` placeholder boss).
3. ✅ Per-room `TileMap` generation (`world/generator.rs`) — the `generate_map` blob ported verbatim
   into `procedural_room_layout` + boss/throne/act-boss/merchant layouts.
4. ✅ Encounter lifecycle (`run/` live, `RunPlugin` in `GameLogicPlugin`): start → load (seeded
   depth-scaled themed roster) → objective (KillAll/Survive/KillMapBoss/Rest) → complete → `MapSelect`
   branch pick → teardown → load next; ActBoss advances the act. Plus the **live scaling driver** (D5
   depth), spawn roles (`MapBoss`), the ThroneRoom curse + Rare-floor kiss, and a Merchant rest node.

**Phase 7.5 — UI layer & presentation backlog** _(complete 2026-07-05 — full scope; the golden master
stayed byte-identical; see §8.10, docs/phase7.5-ui-plan.md)_
1. ✅ In-run HUD (health/XP/cooldowns/stance/class-resource slot/status/objective/boss bar) + shared
   `ui/theme.rs`; `draw_health_bars` skips the player (D5).
2. ✅ Game-over + pause flows (death → `GameState::GameOver` + summary + restart; Esc ⇄ Paused) and the
   `run/systems/reset.rs` run-reset primitive.
3. ✅ Main menu + character select (D1: windowed boot Menu → CharacterSelect → run, replacing
   `auto_start_run`); the Mage is reachable without the debug key.
4. ✅ Visual act-graph map view (over Phase 7's picker) + ThroneRoom curse banner.
5. ✅ Merchant screen + ops (D2: remove + 3-for-1 trade), reusing the Phase-2 uninstall path + the
   kiss picker machinery.
6. ✅ Presentation backlog: zone discs + the cast-VFX bus (Blood Boil nova flash) — closes §8.5's
   nova row + the Phase-6 zone-visuals deferral.

**Phase 8 — Persistence + meta** _(complete 2026-07-06 — full scope incl. the `RunRng` → `ChaCha8Rng`
switch (the phase's one declared golden regen) + Log-In; see §8.11, docs/phase8-plan.md)_
1. ✅ `RunState` serialization (save at every node boundary; load on Resume Run, bit-exact via the
   restored RNG stream).
2. ✅ `MetaState` with hero unlocks (mechanism only — every hero ships unlocked, D3) and scoreboard
   (+ the score formula, §8.1(10)); the main-menu Resume/Scoreboard buttons are live + hero
   unlock/greying is wired on character select.
3. ✅ "Resume Run" from the main menu; player/map spawn moved from `Startup` to `OnEnter(InRun)` (the
   `game/state.rs` TODO), guarded so it seeds the world exactly once.

**Phase 9 — Remaining classes + content pass** _(an ordered sub-phase arc, 9.1–9.7 — see
`docs/phase9-plan.md`; 9.1 complete, §8.12)_
1. Add remaining heroes (each is one RON file + ability/talent RONs). _(9.3 Paladin, 9.4 Druid)_
2. Fill in enemy ability kits per theme + the real per-theme rosters. _(9.6)_
3. Multi-phase boss AI for boss rooms + act boss fights (the merchant screen shipped in Phase 7.5).
   _(9.7, elite multi-ability bosses land in 9.6 first)_

---

## 8. Plan Amendments — Gap Analysis (added 2026-07-05, after Phase 2)

A line-by-line comparison of Mechanics.md against §7 found mechanics with **no phase** and
phases hiding more work than their bullets. Recorded here so scheduling decisions are explicit
rather than discovered mid-phase.

### 8.1 Mechanics with no home in any phase

1. **Auto-cast for passive abilities** — most of the game's kit (Blood Boil, Heart Strike,
   Companion, AMZ, Consecrated Ground, Spinning Hammer, Smite, Flamewrath, …) fires on
   cooldown without input, but the execute pipeline is TriggerAbilityEvent-driven only.
   Needed as early as the L2/3 unlocks. → Schedule as **Phase 3.5** (or fold into Phase 3).
2. **Typed/string params in AbilityDef** — `base_params` is `HashMap<String, f32>`; summons
   need an ability-id ref (companion.ron stores it as a hacky f32), dropped zones need a
   `zone_type` string (commented out in dnd.ron), abilities need "which status do I apply".
   → Prerequisite for Phases 3 (status application) and 6 (zones); do with Phase 3.
3. **Behavior primitives never scheduled** — `projectile` (Phase 1's own leftover debt),
   `channel_while_moving`, `summon` (Companion is BDK **level-1**), `orbiting`,
   `leap_to_target`, and the **movement ability / dash** (`InputSlot::Movement` exists,
   nothing implements it). → Projectile ASAP (Phase 3 uses it for status application tests);
   the rest before the class that needs them (Phase 4/9). ~~The movement ability / dash~~
   **DONE (Phase 9.1, §8.12)** — a `blink` behavior + Shift/Space wired to `InputSlot::Movement`;
   an unbound demonstrator (`dash.ability.ron`) proves it end-to-end. `channel_while_moving`/
   `summon`/`orbiting`/`leap_to_target` remain open — Phase 9.2+ (each lands with its first class
   consumer, per DP2).
4. ~~**Actor stat sheet & CC semantics**~~ — crit %, attack speed, move-speed modifiers, and what
   root/stun/slow actually do to an enemy's movement/AI. resolve_params only covers ability
   params. → New phase alongside status effects (Phase 3.75: "actor stats & CC"). CC semantics
   **DONE (Phase 3, §8.4)**; crit %/attack speed **DONE (Phase 9.1, §8.12)** — a universal stat
   baseline in `resolve_params` + a `RunRng`-driven crit roll + the attack-speed cooldown formula.
5. ~~**Shields/absorbs**~~ (bone shield, ice barrier, Paladin overheal shield) — no system.
   **DONE (Phase 9.1, §8.12)** — the generic `Absorb` pool + `GainShieldEvent`/`apply_shield_gain`.
   The named consumers (bone shield, Ice Barrier, Purgatory, overheal) are still Phase 9.2/9.5/9.3
   content.
6. ~~**Forced movement**~~ (Abomination Limb grip, knockback shockwaves) — no system.
   **DONE (Phase 9.1, §8.12)** — the generic `ForcedImpulse` + `resolve_forced_movement`. Abomination
   Limb's grip and a knockback talent are still Phase 9.2+ content.
7. **Enemy scaling** — "Enemies have their own scaling, independent of the player"
   (Mechanics.md) has no data model (EnemyDef stats are flat) and no phase. → Schedule with
   Phase 5; also the prerequisite for meaningful balance testing.
8. **Enemy projectiles + AMZ blocking** — Phase 5's ranged_caster presupposes projectile
   motion/collision; AMZ's projectile-blocking zone is unscheduled.
9. ~~**UI phase missing entirely**~~ **DONE (Phase 7.5, §8.10).** §2's ui/ module now carries the
   in-run HUD (health/XP/cooldowns/stance/objective/boss bar), the main menu + character select
   (windowed boot Menu → CharacterSelect → run), the game-over + pause flows (death now enters
   `GameState::GameOver` with a summary + restart, not a bare despawn), the visual act-graph map view +
   ThroneRoom curse banner, and a working merchant screen (remove / 3-for-1 trade). Plus the
   presentation backlog: zone discs + the cast-VFX bus (Blood Boil nova flash). **Still deferred to
   Phase 8:** scoreboard + score formula, Resume Run, hero unlock/greying, Log-In profile, and moving
   player/map spawn from `Startup` to `OnEnter(InRun)`. All keyboard-first (headless-testable); the
   screens are verified on Windows.
10. **Smaller unspecified items** — talent-offer rarity weighting; "special events" beyond
    ThroneRoom; `EnemyRarity::Elite` spawn logic; score computation for the scoreboard;
    multi-phase boss design (the plan itself marks the "boss" AI hook TBD — realistically its
    own phase, not one Phase 9 line); Act-3 secret level (defer); audio/art (explicitly out
    of scope until further notice).

### 8.2 Phase-specific corrections

- **Phase 3**: needs an `ApplyStatus` variant in `AbilityEffect`, a hook registry for the
  `on_*_hooks` the status RON files already reference, and the CC semantics from 8.1(4).
- **Phase 4**: much bigger than its two bullets — a second class transitively needs
  projectile + channel + status effects end-to-end, class resources (frost charges), and for
  Druid the enhanced-attack state machine + summons. Choose the second class deliberately
  (Mage exercises stance/projectiles/status with the least extra machinery).
- **Phase 7**: `ActGraph` data exists but no generation function does, even as a stub.
- **Phase 8**: rand 0.8's `SmallRng` does not implement serde — §3.11's "save the RNG state"
  needs either seed + draw-count replay or a switch to `rand_chacha` (serde feature). The
  whole RunState object graph also still lacks serde derives. **Resolved (§8.11, D1):** switched to
  `rand_chacha::ChaCha8Rng`, hand-implementing `Serialize`/`Deserialize` (not the crate's own
  `serde1` feature — its wire format's `u128` word-position can't round-trip through `ron` 0.8). One
  declared golden-master regeneration; the whole object graph gained serde derives in the same phase.
- **Phase 9**: split into content pass (classes/enemies per theme) vs. boss design.

### 8.3 Testing infrastructure (inserted, stages 0–2 complete 2026-07-05)

Headless sim harness (`src/sim/`), logic/presentation plugin split, golden scenario suite +
golden-master campaign baseline, `/compat-check` skill + compat-tester agent. See
docs/testing.md. **Definition of done for every phase from Phase 3 on: the phase lands with
golden scenarios for its mechanic, and the golden-master baseline is regenerated only with a
CHANGELOG entry explaining the behavior change.** Stage 3 (balance arena binary, BotPolicy,
sweep metrics, balance-analyst agent) is scheduled after Phase 5 (enemy scaling in data) and
becomes fully useful after Phase 7 (encounters).

### 8.4 Phase 3 delivered (2026-07-05)

Status effects shipped, absorbing several §8.1 gaps. See `docs/phase3-plan.md` for the full
plan + as-built notes and the CHANGELOG for detail. Resolved here:
- **8.1(1) auto-cast** — folded in (`Activation::AutoCast` + `auto_cast_abilities`; Blood Boil live).
- **8.1(2) typed/string ability params** — solved via the declarative `effects: Vec<EffectSpec>`
  list (incl. `ApplyStatus{status,…}`), superseding the float-only `base_params` for outcomes.
- **8.1(3) projectile primitive** — implemented (motion + collision + on-impact effect delivery).
- **8.1(4) CC semantics** — root/stun immobilize, frostbite slow + damage-taken amp, via generic
  `MoveSpeedModifier`/`DamageTakenModifier`/`Immobilized` components. The *general* actor stat
  sheet (crit/attack-speed) remains deferred.
Still open from §8.1: shields/absorbs (5), forced movement (6), enemy scaling (7), enemy
projectiles/AMZ (8), UI (9). The `StatusHookRegistry` is deferred until a code-driven status
effect needs it.

### 8.5 Phase 3.1 hardening + tech-debt register (2026-07-05)

A post-Phase-3 review landed a hardening batch (CHANGELOG "Phase 3.1"): MovementSet pin,
overlay-freeze event preservation, same-frame status-apply fix, Hurtbox logic/presentation
split, and the test-coverage gaps from phase3-plan §6. The remaining known debt, with the
phase that should absorb each item:

| Debt | Why it can wait | Absorb in |
|---|---|---|
| ~~`Def`-library triplication~~ **RESOLVED (Phase 4).** Generic `DefLibrary<T>` + `DefAsset` + `RonDefLoader<T>` + `register_def_library` in `core/def_library.rs`; the three libraries are now type aliases and `HeroDef` reuses the same path. | — | Done |
| ~~`execute_ready_abilities` mixes trigger validation, faction gather, param resolution, effect application, VFX/projectile spawning, whiff/suppress gates, and cooldown bookkeeping — split around the hook points~~ **RESOLVED (Phase 6).** The first code-driven hook (`blood_boil_dnd_range`) landed; `execute_ready_abilities` now interleaves Pre hooks (resolve→behavior boundary) and Post hooks (after apply), each gated on `ActiveHooks` + registration. `ability/hooks.rs` = `AbilityHook`/`HookContext`/`HookRegistry`. Byte-identical (no registered hook is active on a campaign cast). | — | Done |
| `resolved_cd > 0.0` guard in execute.rs ignores a talent that Overrides cooldown to 0 (Phase 2 note) | No such talent exists; a 0-cd ability would fire every frame and needs a design decision anyway | First cooldown-manipulating talent |
| ~~`suppress_abilities` is parsed but neither resolved into a component nor consumed~~ **RESOLVED (Phase 5).** `resolve_actor_status` folds it into a new `AbilitiesSuppressed` marker; `auto_cast_abilities`, `execute_ready_abilities`, and the hero input/stance systems skip a suppressed caster. Neutral (no shipped content applies stun). | — | Done |
| ~~Travelling projectiles / Blood Boil have no visuals~~ **RESOLVED (Phase 4 + 7.5).** Projectile sprites (`attach_projectile_visuals`) + status tints (`tint_status_effects`) landed Phase 4. The Blood Boil **nova flash** landed Phase 7.5 via the **cast-VFX bus** (`CastVfxEvent`, write-only from `execute_ready_abilities` — byte-identical; drawn as a gizmo ring by `game/vfx.rs`). Zone discs (`attach_zone_visuals`) landed too. **Done** — the logic-side cone-flash path is left on gizmos (migrating it earns nothing and risks the baseline). | — |
| Projectiles ignore walls (no TileMap collision) — a Fireblast shoots through obstacles | **Decided 2026-07-05 (project owner): acceptable for now.** Revisit only if Mage playtesting makes it feel wrong; a fix would be a per-ability `blocked_by_walls` flag + a TileMap check in `move_projectiles` (declared behavior change → baseline regen) | Accepted; revisit during Mage playtesting |
| Per-hero **base-stat application** — `HeroDef.base_stats` (max_health, move_speed) is data-only; `spawn_player` still uses the shared constants, so the Mage plays with the Death Knight's HP/speed | No class HP/speed differentiation is needed for the stance mechanic; keeping it out kept Phase 4 baseline-neutral | When class HP/speed differentiation matters (feel/balance) |
| String ids (`AbilityId`/`StatusEffectId`/`TalentId` = `String`) are cloned per event/frame in hot-ish loops | Scale is tiny; determinism unaffected | Only if profiling ever says so (interning/`Arc<str>`) |
| **Orphaned `AbilityInstance` entities on enemy death / encounter teardown** (found Phase 7.5). Enemy ability instances are separate top-level entities carrying an `owner` field (not real Bevy children), so `enemy_death`'s `despawn(enemy)` and `despawn_encounter_entities` (Enemy/Projectile/Zone/PickUp only) never remove them — they accumulate across a run. Harmless to the golden trace (not a snapshot field), but each orphaned **auto-cast** instance keeps emitting a wasted `TriggerAbilityEvent` every frame (execute then skips it — the dead owner fails the `owners.get`), so wasted work grows O(dead enemies). Only the Phase-7.5 run-reset (`reset.rs`) despawns them all (on restart). | Bounded per run; never affects behavior/determinism, only memory + per-frame waste; the golden master is byte-identical | Cheap fix: despawn an enemy's owned `AbilityInstance` entities in `enemy_death` (+ add them to `despawn_encounter_entities`). Byte-identical (no snapshot field) — do it with the next enemy/perf pass, or now if a run's frame cost is felt |

### 8.6 Phase 4 delivered (2026-07-05)

Hero / stance system + Mage shipped as a **focused vertical slice** (owner decision). See
`docs/phase4-plan.md` for the full plan + as-built notes and the CHANGELOG "Phase 4" section for
detail. Delivered:
- **Hero indirection** — `HeroDef` loader (via the new generic `DefLibrary<T>`), `HeroIdentity` +
  `ActiveStance` on the player, `HeroPlugin` resolving input slots (LMB→Basic, RMB→Special) through
  `HeroDef.stance_slots`; the Phase-1 hardcoded LMB→death_strike stub is deleted.
- **Second class = Mage** (§8.2's recommendation) — Fire/Ice stances binding the Phase-3 Fireblast/
  Frostbolt demonstrators; Q swaps stances and applies the entered stance's swap effect (Boots of
  Fire / Ice Barrier, modeled as statuses). Death Knight formalized as the default `HeroDef`;
  baseline unchanged.
- **Def-library debt paid** (§8.5, was owed "at Phase 4 start") and a **presentation pass**
  (projectile sprites + status tints; nova flash re-filed to §8.5).

Deferred from the full Phase-4 vision, each with a revival trigger (phase4-plan §7): frost-charge
class resource + UI bar, Frost Impale + `channel_while_moving`, dash / movement ability, real
absorb/shield system, code-driven status/ability hooks + the `execute_ready_abilities` split,
`Override(0)` cooldown semantics, per-hero base-stat application, the Blood Boil nova flash,
character-select UI, and full Mage progression content (Blaze, Flamewrath, Frostbite, Frost charge,
Flamestrike, talents — Phase 9 content pass). §8.1 gaps still open: shields/absorbs (5), forced
movement (6), enemy scaling (7), enemy projectiles/AMZ (8), UI (9).

### 8.7 Phase 5 delivered (2026-07-05)

Enemy ability system + AI + a **faction-aware ability engine**, shipped at **full scope** (owner
decision D1 — a ranged caster was included) with a **data-only** scaling model (D2) and a
**unified** enemy/player execution path (D3). See `docs/phase5-plan.md` and the CHANGELOG "Phase 5"
section. **The golden baseline did not move at any of the five steps.** Delivered:
- **Faction-aware engine** — `Faction { Friendly, Hostile }`; target-gathering and projectile
  collision resolve by opposing faction. Enemy casts hit the player; player casts hit enemies;
  one engine for both. (Neutral: same target sets/order for player casts.)
- **`EnemyDef` data-drive** — a live `.enemy.ron` `DefAsset`; `enemy/archetypes.rs` deleted; the 3
  archetypes ported byte-identically; a ranged `spitter` added. AI dispatch is an **`AiBehavior`
  component enum**, deliberately replacing the scaffold's `AiBehaviorRegistry`/`EnemyAiHook`
  trait (a `&mut World`-free hook can't steer via the flow field). Both `EnemyDef` and
  `enemy/behavior.rs` were uncompiled scaffolds; the latter is deleted.
- **Contact melee as a first-class ability** — auto-cast `*_contact` abilities via a `contact_melee`
  behavior; `enemy_attack` + `AttackStats`/`AttackCooldown` deleted. Cadence preserved exactly
  (spawn-with-instances + a `consumes_cooldown_on_whiff` opt-out) ⇒ **baseline byte-identical, no
  regeneration** (the change the phase plan expected to be a declared benign regen turned out neutral).
- **Ranged caster** — `spitter` + `spitter_bolt` + `ranged_caster_ai` (approach → stop at
  `preferred_range` → face the player → fire); enemy projectiles hit the Friendly player. Kept out
  of the golden campaign, so the master is untouched.
- **Enemy scaling — data-only** (resolves §8.1(7)): `EnemyScaling` on `EnemyDef` +
  `resolve_enemy_stats(def, depth)` + a generic `DamageDealtModifier` (mirror of
  `DamageTakenModifier`). No live driver; depth 0 ⇒ base ⇒ neutral. Phase 7 supplies real depth.
- **`suppress_abilities` wired** (pays the §8.5 debt) — `AbilitiesSuppressed` marker + gates in
  auto-cast/execute/hero-input/stance.

§8.1 status after Phase 5: enemy scaling (7) **done (data model)**; enemy **projectiles** (8) **done**
(the AMZ projectile-blocking zone is still open — Phase 6+). Still open from §8.1: shields/absorbs
(5), forced movement (6), UI (9). Deferred from Phase 5 with triggers (phase5-plan §7): `ThemeDef`
loader + theme/encounter spawning + `Elite`/boss spawn roles + a live scaling driver (Phase 7);
multi-phase boss AI + enemy status/DoT kits (Phase 9); AMZ zones (Phase 6+).

### 8.8 Phase 6 delivered (2026-07-05)

Persistent zones + the **code-driven ability-hook system**, shipped at **full scope** (owner decision
D2) with the real hook registry (D1 — not a declarative shortcut). See `docs/phase6-plan.md` and the
CHANGELOG "Phase 6" section. **The golden baseline did not move at any of the six steps.** Delivered:
- **Zone system live** — the scaffold `zone` module (`PersistentZone`/`ZoneAnchor`/`PlayerZonePresence`,
  already written) joined `lib.rs`/`GameLogicPlugin`; maintenance runs at the end of
  `MovementSet::Integrate` (positions settled, presence fresh before combat).
- **Zone-emitting abilities** — new `AbilityDef.zone: Option<ZoneSpec>` (`zone_type` + `anchor`
  {Fixed|FollowCaster} + `blocks_projectiles`) + a `dropped_zone` behavior returning a
  `CastOutcome.zone` request; execute builds the `PersistentZone` from spec + params + the caster's
  `Faction` (the projectile pattern). Content: D&D (buff zone, regen only — `damage_per_second` 0,
  stays RMB `Input`), Tree Conduit (marker), Consecrated Ground (Holy DoT), AMZ (blocking).
- **Code-driven hooks (pays the §8.5 `execute_ready_abilities`-split debt)** — `ability/hooks.rs`:
  `AbilityHook` (`pre`/`post`) + `HookContext` + `HookRegistry`. Execute interleaves Pre hooks
  (resolve→behavior; may mutate `ResolvedParams`) and Post hooks (after apply), gated on the caster's
  `ActiveHooks` **and** registration. This finally consumes the `ActiveHooks` maintained since Phase 2.
  Registered: `blood_boil_dnd_range` (×2 `radius` inside D&D — architecture-plan §4's Talent 3).
  `bone_shield_on_kill` stays inert (its shield system is deferred, §8.1(5)). Byte-identical: no
  registered hook is active on a campaign cast. **This is the deliberate deviation-free realization
  of the §3.4 hook design** (unlike Phases 3/5, which replaced hook-sketches with declarative models,
  Phase 6 built the literal registry because the zone-conditional effect is genuinely code-shaped).
- **Occupant tick effects** — `ZoneEffects` (1 Hz) + `zone_tick_effects`: Holy DoT to opposing-faction
  occupants (Consecrated Ground), regen to the owner inside (D&D). No RNG; neutral where no zone
  exists.
- **AMZ projectile blocking (closes §8.1(8))** — `ZoneBlocksProjectiles` + `block_projectiles_in_zones`
  (before `projectile_collision`): destroys projectiles aimed at the zone's faction inside it, except
  those emitted from inside. The `FollowCaster` anchor mechanism is built + tested; the AMZ-follow
  *talent* is deferred content.

§8.1 status after Phase 6: enemy **projectiles + AMZ blocking (8)** now **fully done**. Still open
from §8.1: shields/absorbs (5) — its `ActiveHook`/Post-hook plumbing now exists (bone shield just
needs the shield system); forced movement (6); UI (9). Deferred from Phase 6 with triggers
(phase6-plan §7): cross-ability zone buffs (Death Strike / Heart Strike inside D&D), Tree Conduit's
enhanced-attack consumer, the AMZ-follow talent, and the bone-shield Post hook — Phase 9 class
content; zone visuals — a presentation pass.

### 8.9 Phase 7 delivered (2026-07-05)

Act graph + room / encounter system, shipped at **full scope** (owner decision D2) with a
**byte-identical golden master** (D1 — no regeneration, like Phases 4–6). See `docs/phase7-plan.md`
and the CHANGELOG "Phase 7" section. Delivered across 7A–7G:
- **The single flat arena became a seeded, branching, themed act of typed encounters.** `ThemeDef`
  is a live `.theme.ron` `DefAsset` (5 themes); `build_act_graph(act, theme, rng)` is a pure,
  seed-deterministic `COLUMNS_PER_ACT = 15` graph (single entry Map / terminal ActBoss / BossRoom /
  1–3-node middle columns with a guaranteed ThroneRoom); `world/generator.rs` produces per-encounter
  rooms (the `generate_map` blob ported verbatim as the Map layout + boss/throne/act-boss/merchant
  layouts).
- **The `run` module is live.** `RunState` + `CurrentEncounter` (in-memory; serde is Phase 8),
  `RunPlugin` in `GameLogicPlugin`, and the lifecycle systems — all `run_if`-gated on a live run, so a
  runless world (the golden campaign) leaves them inert (neutral by construction). Objectives:
  `KillAll` / `Survive` / `KillMapBoss` (the tagged `MapBoss`, ignoring pack adds) / Merchant `Rest`.
  Node selection is a minimal `GameState::MapSelect` keyboard picker (D3); the full visual map view is
  the deferred UI phase.
- **The Phase-5 scaling curve is finally driven (closes §8.1(7) fully).** The seeded encounter spawner
  passes each spawn through `spawn_enemy_from_def(.., depth)` with the node's depth
  (D5 = `(act−1)·COLUMNS_PER_ACT + column`); depth 0 (the Act-1 tutorial) ⇒ base stats (Phase 5's
  neutral promise). Spawn roles draw pack vs. boss from the theme's pools.
- **ThroneRoom = kiss/curse (architecture §6 Q1).** The curse (`RoomModifierDef`, now a `.roommod.ron`
  `DefAsset`) is threaded into `resolve_params`'s `extra_modifiers` for **Hostile casts** (the
  intended mechanism, §3.8) — empty outside a ThroneRoom, so byte-identical to the prior `&[]`. The
  kiss reuses the TalentPicker with a **Rare rarity floor** (`OfferContext::ThroneRoom`). Player-stat
  curses need bespoke consumers (deferred).
- **Windowed-only auto-start (D1).** `GamePlugin` adds a `PostStartup` `auto_start_run`; the headless
  sim never auto-starts, so the campaign stays runless and byte-identical.

§8.1 status after Phase 7: enemy **scaling (7)** now **fully done** (data model + live driver). Still
open from §8.1: shields/absorbs (5), forced movement (6), and the **UI phase (9)** — Phase 7 ships only
the keyboard picker, not the HUD / character-select / visual act-graph map view / merchant screen.
Deferred from Phase 7 with triggers (phase7-plan §7): RunState **serialization/resume** + score (Phase
8, §8.2); **merchant ops** (remove / 3-for-1 — Phase 8/9); the **real per-theme rosters** + multi-phase
boss AI (Phase 9 content — a data edit + boss design); the **visual act-graph map view** (UI phase);
the **player-stat ThroneRoom curses'** bespoke consumers (as each mechanic lands). §8.5: the
`HeroDef.base_stats` per-hero application remains the last open row (the Mage still plays with the DK's
HP/speed).

### 8.10 Phase 7.5 delivered (2026-07-05)

The **UI layer & presentation backlog**, shipped at **full scope** with a **byte-identical golden
master** (no regeneration, like Phases 4–7). See `docs/phase7.5-ui-plan.md` §9 and the CHANGELOG
"Phase 7.5" section. Delivered across 7.5A–7.5G:
- **The whole user-facing surface is live.** `ui/theme.rs` (shared palette + spawn helpers, the
  talent picker refactored onto it) underpins: the **in-run HUD** (`hud.rs` — health/XP bars, stance,
  a `ClassResource` slot, status row, ability slots with cooldown veils + slot labels, objective
  tracker hidden when runless, a `MapBoss` boss bar); the **main menu + character select** (windowed
  boot Menu → CharacterSelect → run, D1 — `GamePlugin` swaps `auto_start_run` → `enter_main_menu`);
  the **game-over + pause** flows; the **visual act-graph map view** (`map_select.rs` upgraded to a
  Slay-the-Spire column view, same input contract) + **ThroneRoom curse banner**; and the **merchant
  screen** (`merchant.rs`).
- **Death → GameOver + restart (was a bare despawn).** `player_death` captures a `GameOverSummary` and
  enters `GameState::GameOver`; `run/systems/reset.rs::reset_and_start_run` is the shared run-reset
  primitive (full teardown incl. orphaned `AbilityInstance` entities → fresh player → reseed → new
  run), driven by a `StartRunRequest` event from the death screen (R) and character-select. Esc toggles
  `InRun ⇄ Paused`.
- **Merchant ops (D2).** The Merchant node opens `GameState::Merchant` instead of auto-completing;
  **remove** reuses the Phase-2 `uninstall_removed_talent`, **3-for-1 trade** removes three and opens a
  rarity-floored picker (reusing the ThroneRoom-kiss machinery via `TradeUpRewardEvent`).
- **Presentation backlog.** Zone discs (`attach_zone_visuals`) + the **cast-VFX bus** (`CastVfxEvent`,
  write-only from `execute_ready_abilities` — byte-identical; the Blood Boil **nova flash** drawn by
  `game/vfx.rs`). Closes the §8.5 nova-flash + Phase-6 zone-visuals deferrals.
- **Debt discovered (filed to §8.5, not fixed).** Building the run-reset teardown surfaced that
  **`AbilityInstance` entities orphan on enemy death / encounter teardown** — nothing but the new
  `reset.rs` despawns them. Byte-identical (not a snapshot field) but a per-run leak + wasted per-frame
  auto-cast triggers; the cheap fix (despawn in `enemy_death`) is filed to §8.5 for the next enemy/perf pass.

§8.1 status after Phase 7.5: **(9) UI phase is done** except the Phase-8 carve-outs below. Still open
from §8.1: shields/absorbs (5), forced movement (6). Deferred from Phase 7.5 with triggers
(phase7.5-plan §7): **scoreboard + score formula**, **Resume Run**, **hero unlock/greying**, **Log-In
profile**, and **moving player/map spawn from `Startup` to `OnEnter(InRun)`** — all Phase 8; mouse-input
handlers, damage numbers / minimap / tooltips, and a settings screen — later UX/art. §8.5: the
`HeroDef.base_stats` per-hero application is now the **last** open register row (the Mage still plays
with the DK's HP/speed — Phase 8/9).

### 8.11 Phase 8 delivered (2026-07-06)

Persistence + meta, shipped at **full scope** with **one declared golden-master regeneration** (D1 —
the `RunRng` algorithm switch; every other step verified byte-identical against the new baseline,
like Phases 4–7.5 verified against the original). See `docs/phase8-plan.md` §11 and the CHANGELOG
"Phase 8" section. Delivered across 8A–8H:

- **The RNG switch (8A, D1 — the one declared regen).** `RunRng` moved from `rand::rngs::SmallRng`
  (no serde support, explicitly not version/platform-stable) to `rand_chacha::ChaCha8Rng` (a
  value-stable, hand-serialized seed+stream+word-position). The golden-master campaign baseline was
  regenerated exactly once for this; `campaign_is_reproducible_within_a_build` stayed green
  throughout (no leaked nondeterminism), and this is now also a **stronger** portability guarantee
  than `SmallRng` gave (docs/testing.md).
- **RunState/MetaState become serializable (8B/8C).** Serde derives across the whole `RunState`
  object graph (`ActGraph`, `LevelUpFlowState`, `TalentOffer`, `StatModifier`/`ModOp`/rarity enums —
  plain data, no manual impls) plus a new `elapsed_secs: f32` run clock (D2). `MetaState`/`RunRecord`
  gained the same; `in_progress_run` became a nested `SavedRun { run, rng }` (human-inspectable RON,
  not opaque bytes). `meta` joined the crate and `GameLogicPlugin` (via `MetaPlugin`, sim-safe —
  in-memory default only); `meta/persistence.rs` is the pure serialize/deserialize + save-path
  resolution layer, with a thin disk-I/O layer that only `GamePlugin` (windowed) touches — the sim
  never sees a filesystem, matching the project's logic/presentation split (§2 architecture rule,
  now applied to persistence).
- **Save cadence + scoring (8D).** `sync_run_state` (run/systems/persistence.rs) closes the gap the
  Phase-7 `RunState` mirror always had — `unlocked_abilities`/`acquired_talents` were never written
  after run-start. It runs at every node boundary (`handle_encounter_complete`'s three exits) and on
  defeat (`player_death`); the non-terminal exits also snapshot `SavedRun` into
  `MetaState.in_progress_run`, and both terminal exits call `record_run_end` (the score formula, D2:
  progress by act/node/level/victory + a never-negative speed bonus against a par time).
- **Resume Run (8E).** `resume_run` mirrors `reset.rs::reset_and_start_run` — teardown, restore the
  exact `RunRng` stream position, respawn, re-grant abilities/talents through the existing idempotent
  event paths, rebuild `CurrentEncounter` from the saved graph position, resume `InRun`. Because the
  RNG stream is exact, the next room's roster is byte-identical to an uninterrupted continuation —
  the D1 payoff, proven by `tests/persistence.rs::resume_continues_the_rng_stream_exactly`. Found and
  fixed along the way: a same-frame talent re-install onto a freshly respawned player could race (or
  be clobbered by) `attach_talent_components`'s own unordered Update turn — `resume_run` now attaches
  those components synchronously, and `attach_talent_components` gained a `Without<AcquiredTalents>`
  guard so it can never overwrite them.
- **MetaState surfaces (8F) + Log-In (8G).** `hero_is_unlocked` (every `HeroDef::MANIFEST` id starts
  unlocked, D3 — the mechanism only) gates character-select greying and pick refusal; a new
  `GameState::Scoreboard` lists `run_history` sorted by score. A new `GameState::Login` splash (D4:
  local-profile only, per §6 Q3) sits ahead of Menu in the windowed boot order.
- **Cleanups (8H).** `spawn_player`/`generate_map`/`init_level_flow` moved `Startup` →
  `OnEnter(GameState::InRun)`, guarded `not(any_with_component::<Player>)` so the one-time boot seed
  doesn't refire on every later re-entry into `InRun` (verified against the TalentPicker↔InRun
  round-trips the golden campaign itself exercises). The orphaned-`AbilityInstance` register row
  (§8.5) is resolved: `enemy_death` and `despawn_encounter_entities` now reap them. Found and fixed
  along the way: `enter_merchant`'s bare `Res<CurrentEncounter>` panicked on the Act-3 victory path
  (Bevy auto-inserts a sync point in the `.chain()`, so `handle_encounter_complete`'s
  `remove_resource` had already applied) — a **pre-existing crash bug**, never exercised before this
  phase added Act-3-victory test coverage; fixed via `Option<Res<_>>`.

§8.1 status after Phase 8: unchanged (5, 6 remain open — no Phase-8 scope touched them). §8.5: the
orphaned-`AbilityInstance` row is **resolved**; `HeroDef.base_stats` per-hero application remains the
**last** open register row (deferred out of Phase 8 by D4-OUT — a second golden regen + a balance
call, → Phase 9). §7 Phase 8's three bullets are now ✅ done, mirroring Phases 4–7.5's flip.

### 8.12 Phase 9.1 delivered (2026-07-07)

The first sub-phase of the Phase-9 content-pass arc (`docs/phase9-plan.md` §2) — five cross-cutting
engine primitives the four unfinished class kits + the real enemy/boss rosters need, built once per
DP2 ("engine primitives land once in 9.1; class-specific behaviors land with their first consumer")
and left **inert until content uses them**. See the CHANGELOG "Phase 9.1" section for full detail.
**The golden master stayed byte-identical at every step** — no shipped ability/talent/enemy
references any of the new stats or components, matching the sub-phase's own DoD. Delivered:

- **Shields/absorbs (closes the §8.1(5) system gap)** — `Absorb` (core/components.rs), draining in
  `apply_damage` between the `DamageTakenModifier` scale and the `Health` write (a pinned scheduling
  point, docs/testing.md); `GainShieldEvent` (a `add_gameplay_event`, like `DamageEvent`/`HealEvent`)
  + `apply_shield_gain`. The **primitive** is done; bone shield / Ice Barrier / Purgatory (the actual
  consumers) are still Phase 9.2/9.5 content.
- **Forced movement (closes the §8.1(6) system gap)** — `ForcedImpulse` + `resolve_forced_movement`,
  first in `MovementSet::Integrate` so grip/knockback still respect the `TileMap` wall-slide and
  override `MovementSet::Intent`'s flow-field/WASD velocity. One primitive, two constructors
  (`toward_point` / `knockback`) — Abomination Limb (9.2) is the first real consumer.
- **Class-resource charges** — `ResourceModel::Charges { max }` + a `Charges` component
  (`gain`/`spend_all`) + `sync_charges_to_class_resource`, a small bridge that mirrors `Charges`
  into the pre-existing (never-inserted-until-now) `ClassResource` the HUD already renders — Mage
  frost charges / Druid enhanced charges (9.4/9.5) light up the bar with zero HUD work. Transient
  per DP3 (not part of `RunState`).
- **Crit % + attack speed (closes the §8.1(4) system gap)** — a universal stat baseline
  (`crit_chance`/`crit_mult`/`attack_speed`, neutral defaults) in `talent/modifier.rs::apply_modifiers`
  so a general passive talent reaches every ability even when its own RON never declares the stat;
  `roll_crit` draws from `RunRng` (never `thread_rng`, per DP5) and — critically — never touches the
  RNG at all when `crit_chance <= 0.0`, which is *why* the golden master stays byte-identical despite
  a live RNG-consuming code path landing in the hot damage-application loop. Attack speed's
  `effective_cd = resolved_cd / (1 + attack_speed)` **also resolves the §8.5 `Override(0)` cooldown-
  guard debt row** (the guard's owning trigger, "the first cooldown-manipulating talent," is now
  moot — the guard is simply gone, replaced by an always-write).
- **Movement-slot dash** — a `blink` behavior + `ForcedImpulseSpawn` (a new `CastOutcome` field
  targeting the caster itself, unlike `zone`/`projectile`'s world-entity spawns); Shift/Space now
  reach `InputSlot::Movement` in `hero/systems/input_slot.rs` (previously wired to nothing). A new
  unbound demonstrator, `assets/abilities/dash.ability.ron`, exercises it; neither shipped hero's
  `stance_slots.movement` binds it yet.

§8.1 status after Phase 9.1: **(4) crit/attack-speed, (5) shields/absorbs, (6) forced movement are
all done** (the general primitives; per-ability consumers are content, landing with their first use
in 9.2+). §8.5: the `resolved_cd > 0.0` / `Override(0)` guard row is **resolved** (removed, not
patched — attack speed's always-write formula makes the guard unnecessary); `HeroDef.base_stats`
remains the **only** open register row (Phase 9.2, the arc's one declared golden-master regen).
Deferred from 9.1 with triggers (phase9-plan §2): the Absorb/ForcedImpulse *consumers* (bone shield,
Ice Barrier, Purgatory, Abomination Limb grip, a knockback talent — Phase 9.2), the Charges
*producers* (Mage frost charges, Druid enhanced/combo — Phase 9.4/9.5), and binding the dash to a
real hero's Movement slot (whichever class's kit calls for it first).

### 8.13 Phase 9.2 delivered (2026-07-07)

The second sub-phase of the Phase-9 content-pass arc: closes out the Blood Death Knight kit —
Companion (a new `summon` behavior), Heart Strike, Abomination Limb, Purgatory, Bone Shield — lands
every remaining BDK talent (Blood Boil/AMZ/class-passive trees, 15 new `.talent.ron` files across
this sub-phase), and applies `HeroDef.base_stats` per-hero. See the CHANGELOG "Phase 9.2" section
for full detail; highlights:

- **`HeroDef.base_stats` now applies (closes the last open §8.5 row).** `MoveSpeed` moved from
  `enemy::components` to `core::components` (now shared); a new deferred `apply_base_stats` system
  mirrors `grant_level_1_abilities`'s async-asset-load pattern, with `respawn_player` applying it
  synchronously on restart/resume so it can't race the resume path's explicit HP restore.
- **Companion is genuinely active** via a new `summon` `AbilityBehavior` + `Minion`/`MinionOwner`/
  `MinionLifetime` — the faction-aware ability engine needed **zero changes** for an independent
  minion attacker to "just work." Two scheduling/logic bugs surfaced and fixed: a grant-vs-execute
  ordering race from adding new unordered systems (pinned `.after(CombatSet::Death)`, mirroring
  `gain_experience`) and a movement/facing-timing oscillation bug (fixed with an engage-range
  hold-position behavior). ~10 pre-existing tests needed `Sim::disable_companion()` to stay isolated
  from its now-real incidental damage.
- **Heart Strike, Abomination Limb, Purgatory, Bone Shield** all landed with their full talent
  trees — a new `grip` behavior (Abomination Limb), a new core `Invulnerable` component + cheat-death
  interceptor (Purgatory), and a kill-counting system reading `ActiveHooks` directly (Bone Shield,
  since a shield grant needs mutable persistent state a read-only Post hook can't hold).
- **AMZ's talent tree is fully implemented**, including a new independent `ZoneSpeedModifier` core
  component (kept separate from the status-owned `MoveSpeedModifier` to avoid a two-writer race) and
  a `follow_caster` param escape hatch letting a talent override an ability's otherwise-static zone
  anchor.
- **Two items deliberately simplified/deferred, both documented at the point of implementation:**
  Bone Shield counts any kill (not Death-Strike-specific — `DamageEvent` carries no ability
  provenance); Blood Boil's health-scaling talent applies flat "bleed" instead of a true
  percent-of-current-health DoT (no such primitive exists yet). Blood Boil's fourth talent
  ("on-death DoT transfer to nearby enemies") is fully deferred — a genuinely new mechanic with no
  RON referencing it yet.
- **`progression/systems/offer.rs`'s hardcoded BDK-only class-passive const is gone** — the offer
  pool now reads `HeroDef.class_passive_pool` dynamically, matching every other def-driven pool.
- **Three isolated golden-master regenerations** (base_stats; Companion; the combined rest-of-kit
  batch), each verified reproducible immediately after. 229 tests passing (was 187).
- **Reproducibility hardening, partial.** After regen #3, `campaign_is_reproducible_within_a_build`
  started failing intermittently (~1 run in 3) — a genuine defect, not something to regenerate
  around. Using Bevy's `ambiguity_detection` schedule-build setting to get an authoritative list
  (rather than guessing), several real, previously-unpinned scheduling races were found and fixed —
  most significantly, `apply_damage`/`apply_heal`/`tick_invulnerability`/`purgatory_cheat_death` all
  independently `.in_set(CombatSet::Apply)` with no order between them despite all four touching
  `Health` essentially every frame; also several `.after(CombatSet::Death)`-anchored systems
  (`spawn_unlocked_ability` vs. `update_minion_lifecycle`; the new health/leech passives vs.
  `install_acquired_talent`/`uninstall_removed_talent`/`apply_base_stats`/`handle_level_up`). These
  fixes are landed and correct. **Reproducibility improved but is not fully closed** — one more
  divergence source remains unidentified (isolated to the player's own position drifting mid-run
  with every enemy bit-identical at that point; RNG/talent-offers and the new AMZ zone-speed
  mechanic were both ruled out by direct testing). Per an explicit product-owner decision
  (2026-07-07), this is landed as tracked debt rather than chased further this session — see the
  new §8.5 row below. **The golden-master baseline is deliberately NOT regenerated** —
  `campaign_matches_golden_baseline` is a known, expected failure against the stale regen-#3
  baseline until this is resolved and a regen #4 lands in a follow-up session.

§8.5 status after Phase 9.2: every row from Phase 9.1 and earlier is resolved (see §8.12/above); one
**new** row opens (below) — the campaign-reproducibility flake. §8.1(5)/(6) (shields, forced
movement) now have real consumers (bone shield, Ice Barrier's absorb is still Phase 9.5;
Abomination Limb's grip). Deferred with triggers: Blood Boil's on-death DoT transfer (needs a new
mechanic — no RON references it); the Charges *producers* (Mage/Druid, Phase 9.4/9.5, unchanged
from 9.1); binding the Movement-slot dash to a real hero.

**New §8.5 row — golden-campaign reproducibility flake (partially fixed, not closed).**
`campaign_is_reproducible_within_a_build` fails intermittently (~1 run in 3) even after the
scheduling fixes above. Known-ruled-out: RNG/talent-offer sequencing (verified byte-identical
across repeated runs), entity-allocation ordering (fixed, verified via the ambiguity checker),
`resolve_zone_speed_bonus`/`ZoneSpeedModifier` (disabled and retested — flake persisted unchanged).
Known signature: the FIRST observable divergence is the player's own position, drifting by roughly
1 unit around the 23-second mark of the 30s scripted campaign, with every enemy's position and
health bit-identical at that same point (ruling out an enemy-side cause). Next steps for whoever
picks this up: (1) re-enable `ScheduleBuildSettings { ambiguity_detection: LogLevel::Error }`
temporarily in `sim/mod.rs` (see this section's own investigation for the pattern) and check the
remaining flagged pairs involving `zone_tick_effects`/`execute_ready_abilities`/`move_projectiles`/
`collect_pickups`/`projectile_collision` (not yet individually pinned — presumed pre-existing and
benign, but not confirmed); (2) consider bisecting against the pre-Phase-9.2 commit to determine
whether this is a latent, pre-existing issue merely first exposed by 9.2's added content, or
something newly introduced; (3) once `campaign_is_reproducible_within_a_build` passes cleanly
across many repeated runs, regenerate the baseline (regen #4) and close this row.

### 8.14 Phase 9.3 delivered (2026-07-08)

The third sub-phase of the Phase-9 content-pass arc: the Paladin, the arc's first brand-new hero
(D2 — Paladin before Druid, to de-risk the shared engine work before Druid's heavier machinery).
Runless-neutral: the golden campaign is the BDK bot and never references any Paladin content, so
its `campaign_matches_golden_baseline` state is unchanged from Phase 9.2 (§8.13's tracked row) —
not investigated further this sub-phase, per an explicit product-owner instruction. See the
CHANGELOG "Phase 9.3" section for full detail; highlights:

- **Three new ability behaviors**, each landing with its first (and so far only) consumer, per DP2:
  `hammer_cleave` (Hammer of Justice — one full-damage primary + a cone-behind-primary cleave, via
  a new `EffectTarget::SecondaryHits` + `EffectSpec::DamageFraction` so a damage talent
  automatically scales the cleave too), `channel_while_moving` (Flash of Light — a new `Channeling`
  component + `ability::systems::channel::tick_channels`, resolving the heal + its talent-gated
  extras only once the channel's `cast_time` elapses; effects are baked at cast start, mirroring
  how a projectile bakes its own), and `orbiting` (Spinning Hammer — a new
  `AbilityContext.elapsed_secs` field drives a stateless, deterministic continuous-rotation
  calculation, sampled via a fast AutoCast cadence rather than a literal physics sweep).
- **The holy-mark read path is finally built** (the marker itself has existed inertly since Phase
  3). A per-target conditional the generic effects pipeline can't express, so both consumers
  (Spinning Hammer's double damage, Hammer of Justice's shockwave talent) are targeted execute.rs
  special-cases backed by a new `is_marked` direct-query helper — the same shape as Phase 9.2's
  `blood_boil_health_scaling`/`abomination_limb_stun`. Smite is the holy-mark GRANT path (reuses
  `nearest_melee` as-is, zero new behavior code).
- **A small, precedented `ZoneEffects` extension** (`slow_status`/`scales_with_occupants`, baked
  from resolved-param flags — the same `follow_caster` escape hatch AMZ's epic talent uses) gives
  Consecrated Ground (promoted from its Phase-6D demonstrator to the real band ability) two more
  talents with no new subsystem — both fields default inert for D&D/AMZ, byte-identical there.
- **A real, previously-undiscovered gap fixed: `init_level_flow` is now hero-aware.** It was still
  hardcoded to the BDK's own band pools regardless of the selected hero — a Phase-2 stub Phase 4
  was supposed to (but never did) source from `HeroDef`, invisible until now because the Mage
  ships with empty band pools and the BDK is the default hero. Paladin is the first hero whose own
  non-empty band pool actually has to reach the real level-up flow, surfacing the bug. Fixed by
  reading the current player's `HeroIdentity` → `HeroDef` band pools when available, falling back
  to the hardcoded BDK consts only for the one call site that fires before any player exists (a
  fallback that is byte-identical to the real data anyway, since the default hero IS the BDK).
- **Three Mechanics talents deliberately deferred, each documented at the point of implementation**,
  the same discipline Phase 9.2 used for Blood Boil's on-death DoT transfer: Hammer of Justice's
  bounce (a chain-bounce shape with no existing analog) and kill-inside-consecrated-ground explosion
  (the same "no ability provenance on DamageEvent" gap bone shield hit); Flash of Light's
  next-Hammer-of-Justice buff (a one-shot cross-ability buff-consumption shape none of
  Modifier/Pre-hook/Post-hook cover — Pre hooks have no `Commands` access to consume a marker, Post
  hooks are deliberately read-only).
- **258 tests total, 257 passing** (was 229) — the one non-passing test is Phase 9.2's already-
  tracked, unchanged `campaign_matches_golden_baseline` divergence; `campaign_is_reproducible_
  within_a_build` stays green. New `tests/paladin.rs` (9 scenarios) plus unit tests for every new
  RON file, the three new behaviors' pure targeting math, and the `DamageFraction`/`SecondaryHits`
  bake logic. Build warning-free.

§8.1 status after Phase 9.3: (3) the `orbiting`/`channel_while_moving` behaviors are now built (their
own listed consumers — Spinning Hammer, Flash of Light — are live); holy-mark consumers (unlisted in
§8.1 but flagged throughout Phase 3–7.5's status-system notes) are now built too. Unchanged/still
open: Ice Barrier's real `Absorb` (Phase 9.5), the Charges *producers* (Mage/Druid, Phase 9.4/9.5),
`leap_to_target` (Phase 9.4), binding the Movement-slot dash to a real hero (still no shipped hero
claims it — Paladin didn't need it), Blood Boil's on-death DoT transfer (Phase 9.2's own deferral,
untouched).

§8.5 status after Phase 9.3: unchanged from §8.13 — the golden-campaign reproducibility flake is
still the only open row (not investigated this sub-phase, by instruction); no new row opens (the
hero-aware band-pool fix was a real bug, not a deliberately-deferred gap, and is now closed, not
tracked).

### 8.15 Phase 9.4 delivered (2026-07-08)

The fourth sub-phase of the Phase-9 content-pass arc: the Druid, the arc's second new hero and — per
the plan's own framing — "the hard class" (two forms, an Enhanced-attack charge state, leaps, a
taunting summon, and a pickup-driven enhancement). Runless-neutral like the Paladin: the golden
campaign is the BDK bot and never references any Druid content. **Verified, not just assumed** — the
pre-existing `campaign_matches_golden_baseline` divergence (§8.5's tracked row, open since Phase 9.2)
was reproduced byte-for-byte on a clean pre-Phase-9.4 checkout (`git stash` + rerun) before any Druid
work landed, confirming this sub-phase moved nothing beyond that already-tracked state. See the
CHANGELOG "Phase 9.4" section for full detail; highlights:

- **Two new ability behaviors**, per DP2: `leap_to_target` (a caster-to-target dash via the Phase-9.1
  `ForcedImpulse` primitive, with two selection modes toggled by a numeric param flag — cursor-nearest
  for Ferocious Bite, highest-health for Primal Pounce — both deterministic, no RunRng) and `bloom` (a
  new `CastOutcome.pickup` signal + `PickUpKind::Enhance`, the first ability whose reward lands on
  pickup-collection rather than at cast time).
- **The Enhanced-attack state machine reuses the Phase-9.1 `Charges` primitive exactly as sketched** —
  a new `Charges::spend_one()` (spend exactly one, distinct from Frost Impale's future `spend_all()`)
  backs Scratch/Ferocious Bite's per-cast Enhanced consumption, each a targeted execute.rs
  special-case (the established per-target-conditional shape, now generalized: `is_marked` became
  `has_status(entity, status_id, ...)`, reused for root/bleed checks across three abilities).
- **`cast_on_enter`** (`StanceSlotMapping`, opt-in per stance): entering a stance fires that stance's
  own Basic ability as a normal `TriggerAbilityEvent` — a genuinely different swap model from the
  Mage's `swap_effect` status buff, kept as a separate opt-in field rather than replacing it.
- **Minion body params generalized from constants to ability data.** Spawn Ent is a second `summon`
  consumer needing a tankier/slower body than the DK's Companion pet; `MINION_HEALTH`/`_SPEED`/
  `_RADIUS` moved from hardcoded constants into each summon ability's own resolved params
  (`companion.ability.ron` now declares the same numbers explicitly — byte-identical). A new
  `taunt_radius` param (0 for Companion) inserts `enemy::components::Taunt` on the minion; a new
  `enemy::systems::taunt::apply_ent_taunt` (before the flow-field follower in `MovementSet::Intent`)
  marks any `MeleeChaser` within range as `Taunted`, which steers straight-line toward the Ent instead
  of the flow field — mirroring the Companion minion's own straight-line seek, for the same reason
  (the shared `FlowField` only ever points toward the player). Contact-range abilities needed zero
  changes — they already hit any opposing-faction target in range, not just the player.
- **A real, previously-inert-primitive scheduling gap found and fixed**, the same class of bug every
  prior sub-phase's first-real-consumer has surfaced: `hero::systems::resource::
  sync_charges_to_class_resource` (Phase 9.1, always inert — no hero carried `Charges` until now) had
  no explicit order against any of its now-three mutators (`execute_ready_abilities`'s Scratch/
  Ferocious Bite spend, `tick_channels`'s Heal grant, `collect_pickups`'s Bloom grant), so Bevy's
  scheduler was free to run the HUD mirror before a same-frame mutation, reading `Charges` a frame
  stale. Fixed by pinning `.after(CombatSet::Damage)` (every mutator lives in or before that set).
- **Deliberate deferrals, documented inline in `Mechanics.md` at each point** (the established
  discipline every sub-phase has used) rather than silently dropped: roughly half of the Druid's
  ~35-talent tree, all keyed to one of a few missing primitives — status-magnitude talents (no
  mechanism scales a `StatusEffectDef`'s own fields, only ability `base_params`), multi-projectile
  spawn (`CastOutcome.projectile` is a single `Option`, not a list), heal-over-time (`TickSpec` only
  deals damage), an aura debuff around an arbitrary (non-player) entity, a minion-owned zone, and
  on-kill ability attribution (the same `DamageEvent`-carries-no-provenance gap Phase 9.2's bone
  shield and Phase 9.3's Hammer of Justice explosion talent both hit). The four class-wide "Passive
  Abilities" (Mega Bleed, Unstable Form, Master of the Forest, form-swap cost) are deferred in full,
  each needing a genuinely new mechanic — `class_passive_pool` stays empty, mirroring Paladin's own
  precedent in Phase 9.3.
- **Tests: 288 total, 287 passing** (was 258; the one non-passing test remains Phase 9.2's own
  tracked, unchanged `campaign_matches_golden_baseline` divergence). New `tests/druid.rs` (10
  scenarios) plus unit tests for every new RON file, `LeapToTarget`'s two selection modes, `Bloom`'s
  pickup signal, and `Charges::spend_one`. Build warning-free.

§8.1 status after Phase 9.4: `leap_to_target` is now built (its listed consumer, Ferocious Bite, is
live); the Charges *producers* named in §8.14 (Druid) are now live — Mage's frost charges remain open
(Phase 9.5). Unchanged/still open: Ice Barrier's real `Absorb` (Phase 9.5), the Movement-slot dash
still has no shipped hero binding (neither Paladin nor Druid needed it).

§8.5 status after Phase 9.4: unchanged — the golden-campaign reproducibility flake is still the only
open row, explicitly reverified (not merely assumed) unchanged by this sub-phase's own work, per the
same instruction Phase 9.3 operated under. No new row opens: the `sync_charges_to_class_resource`
ordering gap was a real, previously-latent bug (inert until a real `Charges` consumer existed) — now
fixed, not tracked.

---

_End of architecture plan. Proceed to implementation only after the open questions in §6 are resolved._
