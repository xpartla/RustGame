// Per-ability runtime state attached to each unlocked ability entity.
//
// Each unlocked ability is a separate Bevy entity, parented to the player entity.
// This allows ECS queries to filter by ability type, stance, and cooldown state
// without storing a Vec on the player component.
//
// Spawn: ability/systems/spawn (called from progression/systems/level_up.rs on UnlockAbilityEvent)
// Query: ability/systems/execute.rs reads these to drive cooldowns and execution

use bevy::prelude::*;
use crate::ability::assets::AbilityId;

/// Marker + identity for a runtime ability entity.
/// The entity has this component plus an AbilityCooldown and optionally a StanceGate.
///
/// Phase 1 stores the owner directly rather than using Bevy hierarchy (ChildOf). The plan's
/// "child of the player" framing is honored logically via `owner`; parent/child wiring can be
/// added later without changing the execution query.
#[derive(Component, Debug, Clone)]
pub struct AbilityInstance {
    /// Links back to the AbilityDef asset for behavior ID, base params, and talent pool.
    pub def_id: AbilityId,
    /// The entity that owns/casts this ability (the player, for now).
    pub owner: Entity,
}

/// Tracks remaining cooldown. The execution system fires when elapsed ≥ cooldown param.
/// Cooldown value comes from ResolvedParams("cooldown") each time the ability fires, so
/// talents that reduce cooldown take effect immediately on the next cast.
#[derive(Component, Debug)]
pub struct AbilityCooldown {
    pub elapsed: f32,
    /// Last resolved cooldown duration. Updated from ResolvedParams on each fire.
    pub duration: f32,
}

impl AbilityCooldown {
    pub fn new(duration: f32) -> Self {
        // Start ready (elapsed == duration) so passive abilities fire immediately.
        Self { elapsed: duration, duration }
    }

    pub fn is_ready(&self) -> bool {
        self.elapsed >= self.duration
    }
}

/// Optional stance gate. If present, this ability only executes when the player's
/// ActiveStance matches. Absent = executes in all stances (e.g. passive abilities).
/// Reserved for the stance system (Phase 4).
#[allow(dead_code)]
#[derive(Component, Debug, Clone)]
pub struct StanceGate(pub String); // StanceId

/// Per-ability state storage for behavior hooks that need persistent counters.
/// Example: bone shield kill counter, frost charge count. Reserved for hooks (Phase 2+).
#[allow(dead_code)]
#[derive(Component, Debug, Default)]
pub struct AbilityHookState(pub std::collections::HashMap<String, f32>);

/// Marker inserted on a player once its HeroDef.level_1_abilities have been granted, so the
/// deferred grant (which waits for the HeroDef asset to load) fires exactly once per player.
#[derive(Component, Debug)]
pub struct Level1Granted;

/// Marker: this entity is a minion spawned by a `summon` ability (Phase 9.2 — Companion). Carries
/// its own `AbilityInstance`/`AbilityCooldown` (mimicking a real, independent caster — the faction-
/// aware ability engine already handles any entity with `WorldPosition`/`Facing`/`Faction`
/// uniformly, so a minion needs no changes to `execute_ready_abilities`). Reaped by
/// `ability::systems::summon::update_minions` on expiry/death, and swept by the encounter/run
/// teardown paths alongside `Enemy`/`Projectile`/`PersistentZone`.
#[derive(Component, Debug)]
pub struct Minion;

/// The entity that summoned this minion. Read by `ability::systems::channel::tick_channels`
/// (Phase 9.4 — Druid Heal's "your heal also heals your Ent" talent, matching minions to owner).
#[derive(Component, Debug)]
pub struct MinionOwner(pub Entity);

/// Remaining lifetime; `update_minions` ticks it down and despawns the minion (+ reaps its owned
/// `AbilityInstance`) on expiry.
#[derive(Component, Debug)]
pub struct MinionLifetime(pub Timer);

/// A multi-frame channel in progress on the caster (Phase 9.3 — Flash of Light; Phase 9.4 — Druid
/// Heal reuses it; later Mage Frost Impale too). Inserted by `execute_ready_abilities` on a
/// `channel_while_moving` cast (instead of applying effects instantly) and resolved by
/// `ability::systems::channel::tick_channels` once `remaining` finishes. Everything the channel
/// needs is baked in at cast time (mirrors how a projectile bakes its effects) — a talent picked
/// up mid-channel doesn't retroactively alter an in-flight one. Fields below Flash of Light's own
/// (Phase 9.3) are Heal-specific talent flags (Phase 9.4); they default to inert (0/false) for
/// every OTHER channel, costing nothing.
#[derive(Component, Debug)]
pub struct Channeling {
    /// Percent of the caster's max health to heal on completion.
    pub heal_percent: f32,
    /// "Overhealed health becomes a shield" (Flash of Light common, unique).
    pub overheal_to_shield: bool,
    /// "Deal X% of amount healed to enemies in a radius around you" (rare). 0 = talent not active.
    pub radiate_percent: f32,
    pub radiate_radius: f32,
    /// "Casting inside consecrated ground makes you radiate, exploding for X damage" (epic,
    /// unique) — pre-resolved at cast start from the talent flag AND zone presence, so completion
    /// only needs to check `> 0.0`. 0 = inactive (talent not active OR wasn't in the zone at cast).
    pub consecrated_radiate_damage: f32,
    /// Druid Heal's "you heal for X% more per bleeding enemy within Y range" (rare, unique).
    /// 0 = talent not active.
    pub bleed_bonus_percent: f32,
    pub bleed_bonus_range: f32,
    /// Druid Heal's "your next attack in animal form is enhanced" (rare, unique) — grants 1
    /// `hero::components::Charges` on completion.
    pub grants_enhanced_charge: bool,
    /// Druid Heal's "your heal also heals your Ent" (rare, unique) — every owned `Minion` also
    /// receives the same flat heal amount as the caster.
    pub heals_ents: bool,
    pub remaining: Timer,
}

/// Event emitted by hero/systems/input_slot.rs when the player presses an input slot.
/// The ability execution system listens for this and fires the matching AbilityInstance.
#[derive(Event, Debug)]
pub struct TriggerAbilityEvent {
    pub ability_id: AbilityId,
    pub owner: Entity,
}

/// Event emitted when an ability is unlocked — by `grant_level_1_abilities` at spawn (Phase-2
/// stub for HeroDef.level_1_abilities) and by progression/systems/level_up.rs for band unlocks.
/// The ability plugin listens and spawns the AbilityInstance entity (idempotently).
#[derive(Event, Debug)]
pub struct UnlockAbilityEvent {
    pub ability_id: AbilityId,
    pub owner: Entity,
}

/// Presentation-only cast-VFX bus (Phase 7.5F). `execute_ready_abilities` *emits* this the moment a
/// cast commits — write-only, no state mutation / RNG / spawns — so the golden campaign trace is
/// byte-identical (a VFX touches no snapshot field). A presentation system (game/vfx.rs) consumes it
/// to draw the flash, so logic never spawns a VFX entity (which would move the baseline). This is the
/// bus the §8.5 "Blood Boil nova flash" item needed. Plain `add_event` — a missed frame is harmless.
#[derive(Event, Debug, Clone)]
pub struct CastVfxEvent {
    pub caster: Entity,
    pub ability_id: AbilityId,
    pub origin: Vec2,
    pub kind: CastVfxKind,
}

/// What flash a cast wants drawn. `Nova` (self-centred novas — Blood Boil / Consecrated Ground)
/// carries the resolved radius; every other cast is `Other` (currently drawn by the existing gizmo
/// paths, so the bus ignores it for now).
#[derive(Debug, Clone, Copy)]
pub enum CastVfxKind {
    Nova { radius: f32 },
    Other,
}
