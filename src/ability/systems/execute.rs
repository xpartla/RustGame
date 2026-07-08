// Drives the per-frame ability execution loop.
//
// Three systems (chained, in CombatSet::Damage so emitted DamageEvents resolve this frame):
//   tick_ability_cooldowns  — advances AbilityCooldown.elapsed for every AbilityInstance
//   auto_cast_abilities     — emits a TriggerAbilityEvent for every ready AutoCast ability
//   execute_ready_abilities — for each TriggerAbilityEvent, fires the matching ready ability
//
// Per fire (Phase 3 generic-effect model):
//   1. resolve_params(base_params × talent modifier stack)   → ResolvedParams
//   2. behavior.resolve(ctx, params)                          → CastOutcome (hits/vfx/projectile)
//   3. resolve_effects(def.effects, params)                   → baked ResolvedEffects
//   4. apply_resolved_effects(...)                            → Damage/Heal/ApplyStatus events
//      (a projectile cast instead spawns the projectile entity carrying the baked effects;
//       projectile/systems/motion.rs applies them on impact via the same shared applier)
//   5. reset cooldown (duration taken from params("cooldown"))
//
// Ability hooks (AbilityDef.hooks) are still unconsumed — they arrive with the first
// code-driven hook (see docs/phase3-plan.md §7).

use bevy::prelude::*;
use std::collections::HashMap;
use crate::ability::assets::{AbilityDef, AbilityLibrary, Activation, HookPhase, ZoneAnchorKind, ZoneSpec};
use crate::ability::behavior::{AbilityContext, BehaviorRegistry, ResolvedParams, Target, VfxShape};
use crate::ability::components::{
    AbilityCooldown, AbilityInstance, CastVfxEvent, CastVfxKind, Channeling, Minion, MinionLifetime, MinionOwner,
    TriggerAbilityEvent,
};
use crate::ability::effects::{apply_resolved_effects, resolve_effects};
use crate::ability::hooks::{HookContext, HookRegistry};
use crate::core::components::{
    AbilitiesSuppressed, Facing, Faction, ForcedImpulse, GridPosition, Health, Hurtbox, MoveSpeed, Velocity,
    WorldPosition,
};
use crate::core::events::{DamageEvent, DamageTag, HealEvent};
use crate::enemy::components::{AiBehavior, Taunt};
use crate::hero::components::Charges;
use crate::pickup::components::PickUpKind;
use crate::pickup::spawn_pickup;
use crate::projectile::components::{ArcHitbox, Lifetime, Projectile, ProjectileMotion, ProjectilePayload};
use crate::status::components::{ApplyStatusEvent, RemoveStatusEvent, StatusEffectInstance};
use crate::run::rng::RunRng;
use crate::run::state::RoomModifiers;
use crate::talent::assets::{StatModifier, TalentDef, TalentLibrary};
use crate::talent::components::{AcquiredTalents, ActiveHooks};
use crate::talent::modifier::resolve_params;
use crate::constants::{MINION_HEALTH, MINION_RADIUS, MINION_SPEED, ZONE_TICK_INTERVAL};
use crate::zone::components::{PersistentZone, PlayerZonePresence, ZoneAnchor, ZoneBlocksProjectiles, ZoneEffects};

/// Advances every ability's cooldown timer toward readiness.
pub fn tick_ability_cooldowns(time: Res<Time>, mut cooldowns: Query<&mut AbilityCooldown>) {
    let dt = time.delta_secs();
    for mut cooldown in &mut cooldowns {
        if cooldown.elapsed < cooldown.duration {
            cooldown.elapsed += dt;
        }
    }
}

/// Emits a TriggerAbilityEvent for every ready AutoCast ability, so passive abilities (Blood Boil,
/// …) fire on cooldown without input. Runs before `execute_ready_abilities` in CombatSet::Damage;
/// execute resets the cooldown when it fires, so exactly one trigger lands per ready period.
pub fn auto_cast_abilities(
    mut triggers: EventWriter<TriggerAbilityEvent>,
    library: Res<AbilityLibrary>,
    defs: Res<Assets<AbilityDef>>,
    suppressed: Query<(), With<AbilitiesSuppressed>>,
    instances: Query<(&AbilityInstance, &AbilityCooldown)>,
) {
    for (instance, cooldown) in &instances {
        if !cooldown.is_ready() {
            continue;
        }
        // A suppressed (stunned) caster does not auto-cast.
        if suppressed.contains(instance.owner) {
            continue;
        }
        let Some(def) = library.get(&instance.def_id).and_then(|h| defs.get(h)) else {
            continue;
        };
        if def.activation == Activation::AutoCast {
            triggers.write(TriggerAbilityEvent {
                ability_id: instance.def_id.clone(),
                owner: instance.owner,
            });
        }
    }
}

/// Fires the ability matching each TriggerAbilityEvent, if its cooldown is ready and its
/// AbilityDef has finished loading.
pub fn execute_ready_abilities(
    mut commands: Commands,
    mut triggers: EventReader<TriggerAbilityEvent>,
    mut damage_events: EventWriter<DamageEvent>,
    mut heal_events: EventWriter<HealEvent>,
    mut status_events: EventWriter<ApplyStatusEvent>,
    mut cast_vfx: EventWriter<CastVfxEvent>,
    // Grouped into one tuple SystemParam to stay under Bevy's 16-param-per-system limit. `time`
    // (Phase 9.3) feeds `AbilityContext.elapsed_secs` for `Orbiting`'s continuous rotation; `marks`
    // is the holy-mark read path (Spinning Hammer's double damage, Hammer of Justice's shockwave
    // talent) — both targeted execute.rs special-cases, like `blood_boil`'s below, since a per-
    // target conditional doesn't fit the generic Pre/Post hook or effects pipeline. `charges`
    // (Phase 9.4) is the Druid's Enhanced-attack state, mutably spent by Scratch/Ferocious Bite's
    // own special-cases below. `remove_status_events` (Phase 9.5) is Flamewrath's blaze-consumption
    // special-case.
    (registry, hook_registry, zone_presence, room_mods, mut rng, time, marks, mut charges, mut remove_status_events): (
        Res<BehaviorRegistry>,
        Res<HookRegistry>,
        Res<PlayerZonePresence>,
        Res<RoomModifiers>,
        ResMut<RunRng>,
        Res<Time>,
        Query<&StatusEffectInstance>,
        Query<&mut Charges>,
        EventWriter<RemoveStatusEvent>,
    ),
    library: Res<AbilityLibrary>,
    defs: Res<Assets<AbilityDef>>,
    talent_defs: Res<Assets<TalentDef>>,
    talent_library: Res<TalentLibrary>,
    owners: Query<(&WorldPosition, &Facing, &Faction, &Health, Option<&AcquiredTalents>, Option<&ActiveHooks>)>,
    // Candidate targets are actors only — never zones (which also carry WorldPosition + Faction, so
    // without this guard a friendly zone could be gathered/targeted by an enemy's cast).
    // `Option<&AiBehavior>` (Phase 9.2) feeds `Target.is_ranged` for Abomination Limb's
    // ranged-only grip talent; `&Health` (Phase 9.4) feeds `Target.health` for Primal Pounce's
    // highest-health leap target — both folded into this query rather than a separate one to stay
    // under Bevy's per-system param cap.
    actors: Query<(Entity, &WorldPosition, &Faction, Option<&AiBehavior>, &Health), Without<PersistentZone>>,
    suppressed: Query<(), With<AbilitiesSuppressed>>,
    mut instances: Query<(&AbilityInstance, &mut AbilityCooldown)>,
) {
    // Gather candidate targets once per faction for all abilities fired this frame. A cast is
    // handed the list opposing its caster's faction (§Phase 5 faction-aware targeting).
    let mut friendly: Vec<Target> = Vec::new();
    let mut hostile: Vec<Target> = Vec::new();
    for (entity, pos, faction, ai, health) in &actors {
        let is_ranged = matches!(ai, Some(AiBehavior::RangedCaster { .. }));
        let t = Target { entity, pos: pos.0, is_ranged, health: health.current };
        match faction {
            Faction::Friendly => friendly.push(t),
            Faction::Hostile => hostile.push(t),
        }
    }
    let targets_for = |opposing: Faction| -> &[Target] {
        match opposing {
            Faction::Friendly => &friendly,
            Faction::Hostile => &hostile,
        }
    };

    // Fallback for owners without a talent list (e.g. non-player casters) — an empty stack.
    let no_talents = AcquiredTalents::default();

    for trigger in triggers.read() {
        let Ok((owner_pos, owner_facing, owner_faction, owner_health, acquired_opt, active_hooks)) = owners.get(trigger.owner) else {
            continue;
        };
        // A suppressed (stunned) caster cannot fire, even via a queued trigger.
        if suppressed.contains(trigger.owner) {
            continue;
        }
        let acquired = acquired_opt.unwrap_or(&no_talents);
        let opposing = owner_faction.opposing();
        let targets = targets_for(opposing);
        let has_aim = owner_facing.0.length_squared() >= 1e-6;

        for (instance, mut cooldown) in &mut instances {
            if instance.owner != trigger.owner || instance.def_id != trigger.ability_id {
                continue;
            }
            if !cooldown.is_ready() {
                break;
            }

            let Some(handle) = library.get(&instance.def_id) else {
                break;
            };
            let Some(def) = defs.get(handle) else {
                break; // asset still loading
            };
            let Some(behavior) = registry.get(&def.behavior) else {
                warn!(
                    "ability '{}' uses unregistered behavior '{}' — skipping",
                    instance.def_id, def.behavior
                );
                break;
            };
            // Aim-dependent shapes (cone, projectile) need a non-zero facing; self-centred shapes
            // (nova) do not. Skip without consuming the cooldown when a needs-aim cast has no aim.
            if behavior.needs_aim() && !has_aim {
                break;
            }

            // ThroneRoom curse (Phase 7F): a room's `RoomModifiers` are applied to HOSTILE casts only
            // (the curse makes the fight harder — e.g. "enemies deal double damage"); player casts are
            // untouched. Empty except inside a ThroneRoom, so this is byte-identical to the prior `&[]`.
            let extra_modifiers: &[StatModifier] = if matches!(owner_faction, Faction::Hostile) {
                &room_mods.0
            } else {
                &[]
            };
            let mut params = resolve_params(
                &instance.def_id,
                &def.base_params,
                acquired,
                &talent_defs,
                &talent_library,
                extra_modifiers,
            );

            // Innate Pre hooks (Phase 9.2): an ability's own base-kit identity — e.g. Heart Strike's
            // missing-health damage scaling — always runs if registered, no ActiveHooks gate. See
            // AbilityDef.innate_hooks's doc comment for why this is a separate list from `hooks`.
            for (phase, hook_id) in &def.innate_hooks {
                if *phase == HookPhase::Pre {
                    if let Some(hook) = hook_registry.get(hook_id) {
                        hook.pre(
                            &HookContext { caster: trigger.owner, zones: &zone_presence, health: owner_health },
                            &mut params,
                        );
                    }
                }
            }

            // Pre hooks (Phase 6 — the resolve→behavior boundary): a behavior-rewriting talent the
            // caster has acquired may mutate the resolved params before the behavior resolves — e.g.
            // `blood_boil_dnd_range` doubles `radius` while the caster stands in D&D. Runs only for
            // hooks BOTH installed on the caster (ActiveHooks) AND registered in HookRegistry; an
            // un-acquired or not-yet-implemented hook (e.g. bone_shield) is skipped, zero cost.
            if let Some(active) = active_hooks {
                for (phase, hook_id) in &def.hooks {
                    if *phase == HookPhase::Pre && active.contains(hook_id) {
                        if let Some(hook) = hook_registry.get(hook_id) {
                            hook.pre(
                                &HookContext { caster: trigger.owner, zones: &zone_presence, health: owner_health },
                                &mut params,
                            );
                        }
                    }
                }
            }

            let ctx = AbilityContext {
                owner: trigger.owner,
                origin: owner_pos.0,
                // Non-zero for needs-aim casts (gated above); zero is fine for self-centred shapes.
                facing: owner_facing.0.normalize_or_zero(),
                targets,
                elapsed_secs: time.elapsed_secs(),
            };
            let outcome = behavior.resolve(&ctx, &params);
            // Whiff gate (Phase 5): behaviors like contact_melee don't spend their cooldown when
            // they resolve nothing, so an out-of-range enemy stays charged. Break without applying
            // effects or resetting the cooldown. Aimed/nova casts keep the default (whiff commits).
            if !behavior.consumes_cooldown_on_whiff()
                && outcome.hits.is_empty()
                && outcome.projectile.is_none()
            {
                break;
            }

            // Channel (Phase 9.3 — Flash of Light): defer every effect to channel completion
            // instead of applying instantly. The talent-gated extras (overheal→shield / radiate /
            // the consecrated-ground epic) are baked from ActiveHooks + zone presence AT CAST
            // START, mirroring how a projectile bakes its effects at cast time — a talent picked up
            // mid-channel doesn't retroactively alter an in-flight channel (the projectile
            // precedent). `tick_channels` (ability/systems/channel.rs) resolves it later.
            if let Some(channel_spawn) = outcome.channel {
                let has_hook = |id: &str| active_hooks.map(|h| h.contains(id)).unwrap_or(false);
                let consecrated_radiate_damage = if has_hook("flash_of_light_consecrated_radiate")
                    && zone_presence.is_inside("consecrated_ground")
                {
                    params.get("consecrated_radiate_damage")
                } else {
                    0.0
                };
                // Druid Heal's own talent-gated extras (Phase 9.4), same "bake at cast start" rule.
                let bleed_bonus_percent = if has_hook("heal_bleed_bonus") { params.get("bleed_bonus_percent") } else { 0.0 };
                // Mage Frost Impale's icicle (Phase 9.5): baked from the ability's own resolved
                // params exactly like every other field above — Flash of Light/Heal declare none of
                // these keys, so they resolve to 0.0 and stay inert with no def_id check needed.
                let icicle_speed = params.get("speed");
                let icicle_lifetime = if icicle_speed > 1e-3 { params.get("range") / icicle_speed } else { 2.0 };
                commands.entity(trigger.owner).insert(Channeling {
                    heal_percent: params.get("heal_percent"),
                    overheal_to_shield: has_hook("flash_of_light_overheal_shield"),
                    radiate_percent: if has_hook("flash_of_light_radiate") { params.get("radiate_percent") } else { 0.0 },
                    radiate_radius: params.get("radiate_radius"),
                    consecrated_radiate_damage,
                    bleed_bonus_percent,
                    bleed_bonus_range: params.get("bleed_bonus_range"),
                    grants_enhanced_charge: has_hook("heal_grants_enhanced"),
                    heals_ents: has_hook("heal_heals_ents"),
                    icicle_damage: params.get("damage"),
                    icicle_charge_damage_percent: params.get("frost_charge_damage_percent"),
                    icicle_radius: params.get("radius"),
                    icicle_speed,
                    icicle_pierce: params.get("pierce").max(0.0) as u32,
                    icicle_lifetime,
                    icicle_crit_chance: params.get("crit_chance"),
                    icicle_crit_mult: params.get("crit_mult"),
                    remaining: Timer::from_seconds(channel_spawn.cast_time, TimerMode::Once),
                });
                cast_vfx.write(CastVfxEvent {
                    caster: trigger.owner,
                    ability_id: instance.def_id.clone(),
                    origin: outcome.origin,
                    kind: CastVfxKind::Other,
                });
                cooldown.elapsed = 0.0;
                let resolved_cd = params.get("cooldown");
                let attack_speed = params.get("attack_speed");
                cooldown.duration = resolved_cd / (1.0 + attack_speed).max(0.05);
                break;
            }

            let resolved = resolve_effects(&def.effects, &params);
            // Instant hits (cone/nova). Empty for a pure projectile cast (delivery is deferred).
            // Crit rolls (Phase 9.1) draw from RunRng, never thread_rng — only when a target
            // ability's resolved crit_chance > 0.0 (see roll_crit's byte-identical guarantee).
            apply_resolved_effects(
                &mut damage_events,
                &mut heal_events,
                &mut status_events,
                &mut rng,
                trigger.owner,
                &outcome.hits,
                outcome.primary,
                &resolved,
            );

            // Cast-VFX bus (Phase 7.5F): announce the committed cast for the presentation layer to
            // flash. Write-only — no state/RNG/spawn — so the golden campaign trace is unchanged.
            // Self-novas (Blood Boil) carry their resolved radius for a fading ring; every other cast
            // is `Other` (its VFX still comes from the existing gizmo paths).
            let vfx_kind = if def.behavior == "self_nova" {
                CastVfxKind::Nova { radius: params.get("radius") }
            } else {
                CastVfxKind::Other
            };
            cast_vfx.write(CastVfxEvent {
                caster: trigger.owner,
                ability_id: instance.def_id.clone(),
                origin: outcome.origin,
                kind: vfx_kind,
            });

            // Shape VFX (melee cone flash), reusing the prototype's gizmo entity path.
            if let Some(VfxShape::Cone { radius, half_angle, forward, lifetime }) = outcome.vfx {
                commands.spawn((
                    Projectile,
                    WorldPosition(outcome.origin),
                    ArcHitbox { radius, half_angle },
                    Facing(forward),
                    Lifetime { timer: Timer::from_seconds(lifetime, TimerMode::Once) },
                ));
            }

            // Dropped persistent zone (D&D / Consecrated Ground / AMZ / Tree Conduit). The behavior
            // resolved the drop point; the ability's `zone` spec + resolved params supply the type,
            // anchor, radius/duration, and occupant effects. Carries the caster's Faction so occupant
            // damage targets the opposing side and blocking protects the caster's side.
            if let (Some(spawn), Some(spec)) = (outcome.zone, def.zone.as_ref()) {
                spawn_dropped_zone(&mut commands, spec, &params, spawn.center, trigger.owner, *owner_faction);
            }

            // Collectible pickup drop (Phase 9.4 — Bloom). The behavior resolved only the drop
            // point; "bloom_charges" (from the ability's own resolved params) is how many Enhanced
            // charges it grants on pickup — a talent can raise it via a plain numeric Modifier
            // (`bloom_extra_charge_rare`), no special-case needed.
            if outcome.pickup.is_some() {
                let charges = params.get("bloom_charges").max(1.0) as u32;
                spawn_pickup(&mut commands, outcome.origin, PickUpKind::Enhance(charges));
            }

            // Forced-movement impulse (Phase 9.1 — the Movement-slot dash/blink). Applied directly
            // to the caster, not the world, unlike the zone/projectile spawns above.
            if let Some(spawn) = outcome.forced_impulse {
                commands.entity(trigger.owner).insert(ForcedImpulse {
                    velocity: spawn.velocity,
                    timer: Timer::from_seconds(spawn.duration, TimerMode::Once),
                });
            }

            // Grip (Phase 9.2 — Abomination Limb): a forced pull toward the caster on each gripped
            // target, distinct from `forced_impulse` above (which targets the CASTER itself).
            for grip in &outcome.grip_targets {
                commands.entity(grip.target).insert(ForcedImpulse {
                    velocity: grip.velocity,
                    timer: Timer::from_seconds(grip.duration, TimerMode::Once),
                });
            }
            // "After gripping, stun for X seconds" (the `abomination_limb_stun_rare` talent) — a
            // targeted special-case (like blood_boil_spawns_dnd above): Post hooks are read-only
            // and can't emit follow-up events, so this isn't generalized through the HookRegistry.
            if !outcome.grip_targets.is_empty() {
                if let Some(active) = active_hooks {
                    if active.contains("abomination_limb_stun") {
                        for grip in &outcome.grip_targets {
                            status_events.write(ApplyStatusEvent {
                                target: grip.target,
                                source: trigger.owner,
                                effect_id: "stun".to_string(),
                                stacks: 1,
                            });
                        }
                    }
                }
            }

            // Minion spawn (Phase 9.2 — Companion; Phase 9.4 — Spawn Ent). The behavior resolved
            // only the drop point; the ability's `summon` spec supplies which ability the minion
            // mimics. Body stats (health/speed/radius) are read from the SUMMON ability's own
            // resolved params rather than the shared MINION_* constants (Phase 9.4 — Spawn Ent's
            // Ent is a much tankier body than Companion's pet; falling back to the Phase-9.2
            // constants keeps `companion.ability.ron` byte-identical if it ever omitted them, but
            // it now declares them explicitly). "taunt_radius" (Phase 9.4, Ent only) inserts a
            // `Taunt` component read by `enemy::systems::taunt::apply_ent_taunt`; absent/0 for
            // Companion, so it costs nothing there. Carries the caster's own Faction (mirrors
            // spawn_dropped_zone) so a future Hostile summoner's minion would fight for the right side.
            if let (Some(_), Some(spec)) = (&outcome.summon, def.summon.as_ref()) {
                let minion_health = if params.get("minion_health") > 0.0 { params.get("minion_health") } else { MINION_HEALTH };
                let minion_speed = if params.get("minion_speed") > 0.0 { params.get("minion_speed") } else { MINION_SPEED };
                let minion_radius = if params.get("minion_radius") > 0.0 { params.get("minion_radius") } else { MINION_RADIUS };
                let mut minion_cmds = commands.spawn((
                    Minion,
                    MinionOwner(trigger.owner),
                    MinionLifetime(Timer::from_seconds(params.get("companion_duration"), TimerMode::Once)),
                    Health::new(minion_health),
                    *owner_faction,
                    WorldPosition(outcome.origin),
                    GridPosition::from_world(outcome.origin),
                    Velocity::default(),
                    Facing(Vec2::default()),
                    MoveSpeed(minion_speed),
                    Hurtbox { radius: minion_radius },
                ));
                let taunt_radius = params.get("taunt_radius");
                if taunt_radius > 0.0 {
                    minion_cmds.insert(Taunt { radius: taunt_radius });
                }
                let minion = minion_cmds.id();
                commands.spawn((
                    AbilityInstance { def_id: spec.mimic.clone(), owner: minion },
                    AbilityCooldown::new(0.0),
                ));
            }

            // Travelling projectile: spawn it carrying the baked effects for on-impact delivery.
            // Mage-specific impact special-cases (Phase 9.5), baked here (not read again on
            // impact) mirroring how `effects` itself is baked — a talent picked up mid-flight can't
            // retroactively alter an in-flight shot: Frostbolt's innate frost-charge-on-repeat-
            // frostbite (a resolved-param flag, `grants_frost_charge`, absent/0 for every other
            // projectile) and Fireblast's "explodes on impact" unique talent (an ActiveHooks flag).
            if let Some(spawn) = outcome.projectile {
                let explode_on_impact = active_hooks
                    .map(|h| h.contains("fireblast_explode_on_impact"))
                    .unwrap_or(false)
                    .then(|| (params.get("explode_damage"), params.get("explode_radius")));
                commands.spawn((
                    Projectile,
                    WorldPosition(outcome.origin),
                    ProjectileMotion {
                        velocity: spawn.velocity,
                        radius: spawn.radius,
                        pierce_remaining: spawn.pierce,
                    },
                    ProjectilePayload {
                        source: trigger.owner,
                        target_faction: opposing,
                        effects: resolved.clone(),
                        already_hit: Vec::new(),
                        grants_frost_charge_on_frostbitten: params.get("grants_frost_charge") > 0.5,
                        explode_on_impact,
                    },
                    Lifetime { timer: Timer::from_seconds(spawn.lifetime, TimerMode::Once) },
                ));
            }

            // Post hooks (Phase 6 — after effects apply): react to the resolved cast outcome. Bone
            // Shield's kill-counting (Phase 9.2) is NOT wired through here — it needs mutable,
            // persistent counter state and a conditional `GainShieldEvent`, which the read-only
            // `AbilityHook::post` signature can't express; it's its own system
            // (`ability::systems::bone_shield::bone_shield_on_kill`) instead.
            if let Some(active) = active_hooks {
                for (phase, hook_id) in &def.hooks {
                    if *phase == HookPhase::Post && active.contains(hook_id) {
                        if let Some(hook) = hook_registry.get(hook_id) {
                            hook.post(
                                &HookContext { caster: trigger.owner, zones: &zone_presence, health: owner_health },
                                &outcome,
                            );
                        }
                    }
                }
            }

            // "Blood Boil automatically spawns D&D" (the `bdk_passive_blood_boil_spawns_dnd`
            // talent). A targeted special-case, not a generalized Post-hook-requests-a-spawn
            // mechanism (§8.1(3)'s Post hooks are deliberately read-only) — the only shipped talent
            // that needs one; a second consumer would be the trigger to generalize this.
            if instance.def_id == "blood_boil" {
                if let Some(active) = active_hooks {
                    if active.contains("bdk_blood_boil_spawns_dnd") {
                        spawn_dropped_zone(
                            &mut commands,
                            &ZoneSpec { zone_type: "death_and_decay".to_string(), anchor: ZoneAnchorKind::Fixed, blocks_projectiles: false },
                            &ResolvedParams(HashMap::from([
                                ("zone_radius".to_string(), 80.0),
                                ("zone_duration".to_string(), 8.0),
                                ("damage_per_second".to_string(), 0.0),
                                ("regen_percent_per_second".to_string(), 0.5),
                            ])),
                            outcome.origin,
                            trigger.owner,
                            *owner_faction,
                        );
                    }
                    // "Blood Boil applies a DoT scaling with the target's current health"
                    // (`blood_boil_health_scaling_rare`) — simplified to plain "bleed" on every
                    // hit; see blood_boil.ability.ron's doc comment on the simplification.
                    if active.contains("blood_boil_health_scaling") {
                        for hit in &outcome.hits {
                            status_events.write(ApplyStatusEvent {
                                target: hit.entity,
                                source: trigger.owner,
                                effect_id: "bleed".to_string(),
                                stacks: 1,
                            });
                        }
                    }
                }
            }

            // Spinning Hammer (Phase 9.3): the holy-mark consumer — deals double damage to marked
            // targets. A per-target conditional the generic effects pipeline can't express (every
            // Damage effect applies one uniform amount to its whole hit set), so this tops up an
            // extra `damage` on top of the base AllHits damage already applied above, only for
            // marked hits — same shape as Blood Boil's special-cases. The stun-on-hit talent rides
            // along here too.
            if instance.def_id == "spinning_hammer" {
                let stun_active = active_hooks.map(|h| h.contains("spinning_hammer_stun")).unwrap_or(false);
                let damage = params.get("damage");
                for hit in &outcome.hits {
                    if has_status(hit.entity, "holy_mark", &marks) {
                        damage_events.write(DamageEvent {
                            target: hit.entity,
                            amount: damage,
                            source: trigger.owner,
                            tags: vec![DamageTag::Holy],
                        });
                    }
                    if stun_active {
                        status_events.write(ApplyStatusEvent {
                            target: hit.entity,
                            source: trigger.owner,
                            effect_id: "stun".to_string(),
                            stacks: 1,
                        });
                    }
                }
            }

            // Hammer of Justice (Phase 9.3): "if it strikes a target affected by holy mark, emit a
            // shockwave from your character, dealing X damage and pushing enemies back"
            // (`hammer_of_justice_shockwave_rare`) — a holy-mark consumer + the Phase-9.1 forced-
            // movement primitive (knockback), centred on the CASTER (not the struck target).
            if instance.def_id == "hammer_of_justice" {
                if active_hooks.map(|h| h.contains("hammer_of_justice_shockwave")).unwrap_or(false) {
                    if let Some(primary) = outcome.primary {
                        if has_status(primary.entity, "holy_mark", &marks) {
                            let radius = params.get("shockwave_radius");
                            let damage = params.get("shockwave_damage");
                            let knock_speed = params.get("shockwave_knock_speed");
                            for t in targets {
                                if t.pos.distance(outcome.origin) <= radius {
                                    damage_events.write(DamageEvent {
                                        target: t.entity,
                                        amount: damage,
                                        source: trigger.owner,
                                        tags: vec![DamageTag::Holy],
                                    });
                                    commands.entity(t.entity).insert(ForcedImpulse::knockback(
                                        (t.pos - outcome.origin).normalize_or_zero(),
                                        knock_speed,
                                        0.3,
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            // Smite (Phase 9.3): "after smiting, create consecrated ground under the target"
            // (`smite_spawns_consecrated_rare`) + "holy mark affects all enemies in a radius around
            // the target" (`smite_mark_radius_epic`) — both targeted special-cases keyed off this
            // cast's own primary hit, mirroring Blood Boil's spawn-on-cast special-case.
            if instance.def_id == "smite" {
                if let (Some(active), Some(primary)) = (active_hooks, outcome.primary) {
                    if active.contains("smite_spawns_consecrated") {
                        spawn_dropped_zone(
                            &mut commands,
                            &ZoneSpec { zone_type: "consecrated_ground".to_string(), anchor: ZoneAnchorKind::Fixed, blocks_projectiles: false },
                            &ResolvedParams(HashMap::from([
                                ("zone_radius".to_string(), 50.0),
                                ("zone_duration".to_string(), 4.0),
                                ("damage_per_second".to_string(), 3.0),
                                ("regen_percent_per_second".to_string(), 0.0),
                            ])),
                            primary.pos,
                            trigger.owner,
                            *owner_faction,
                        );
                    }
                    if active.contains("smite_mark_radius") {
                        let radius = params.get("mark_radius");
                        for t in targets {
                            if t.pos.distance(primary.pos) <= radius {
                                status_events.write(ApplyStatusEvent {
                                    target: t.entity,
                                    source: trigger.owner,
                                    effect_id: "holy_mark".to_string(),
                                    stacks: 1,
                                });
                            }
                        }
                    }
                }
            }

            // Scratch (Phase 9.4 — Druid): base damage lands via `effects` above (AllHits). This
            // block layers on the ability's own innate identity that a fixed-literal `EffectSpec`
            // can't express: bonus damage to rooted/bleeding hits (a per-target conditional — same
            // shape as Spinning Hammer's holy-mark top-up), and the Enhanced-attack consumer —
            // spending one `Charges` (if any) applies "bleed" to the `bleed_target_count` nearest
            // hits (default effectively unlimited; the `scratch_bleed_closest_common` talent lowers
            // it to a specific N).
            if instance.def_id == "scratch" {
                let root_bonus_pct = params.get("root_bonus_damage_percent");
                let bleed_bonus_pct = params.get("bleeding_bonus_damage_percent");
                for hit in &outcome.hits {
                    if root_bonus_pct > 0.0 && has_status(hit.entity, "root", &marks) {
                        damage_events.write(DamageEvent {
                            target: hit.entity,
                            amount: params.get("damage") * root_bonus_pct / 100.0,
                            source: trigger.owner,
                            tags: vec![DamageTag::Physical],
                        });
                    }
                    if bleed_bonus_pct > 0.0 && has_status(hit.entity, "bleed", &marks) {
                        damage_events.write(DamageEvent {
                            target: hit.entity,
                            amount: params.get("damage") * bleed_bonus_pct / 100.0,
                            source: trigger.owner,
                            tags: vec![DamageTag::Physical],
                        });
                    }
                }
                if !outcome.hits.is_empty() {
                    if let Ok(mut charges) = charges.get_mut(trigger.owner) {
                        if charges.spend_one() {
                            let bleed_target_count = params.get("bleed_target_count").max(1.0) as usize;
                            let mut enhanced_hits = outcome.hits.clone();
                            enhanced_hits.sort_by(|a, b| {
                                let da = a.pos.distance_squared(outcome.origin);
                                let db = b.pos.distance_squared(outcome.origin);
                                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                            });
                            for hit in enhanced_hits.into_iter().take(bleed_target_count) {
                                status_events.write(ApplyStatusEvent {
                                    target: hit.entity,
                                    source: trigger.owner,
                                    effect_id: "bleed".to_string(),
                                    stacks: 1,
                                });
                            }
                        }
                    }
                }
            }

            // Ferocious Bite (Phase 9.4 — Druid): base damage to the leap target lands via
            // `effects` (PrimaryHit) above. "Always critically strikes if bleeding" (Mechanics) is
            // DETERMINISTIC (DP5, phase9-plan.md — no RunRng roll needed since it isn't chance-
            // based): a flat top-up bringing the total to `damage * bleed_crit_mult`, mirroring
            // Spinning Hammer's holy-mark top-up. Enhanced: cleaves in a circle — approximated as
            // centred on the PRIMARY's position (the caster's own post-leap position isn't known
            // this frame; the leap's `ForcedImpulse` resolves next frame in MovementSet::Integrate,
            // and the primary is where the caster is about to land) — applying bleed to every other
            // target within `cleave_radius` (a documented simplification of "X% of damage as bleed,"
            // which needs a status-magnitude primitive that doesn't exist yet — see CHANGELOG).
            if instance.def_id == "ferocious_bite" {
                if let Some(primary) = outcome.primary {
                    let bleed_crit_mult = params.get("bleed_crit_mult");
                    if bleed_crit_mult > 1.0 && has_status(primary.entity, "bleed", &marks) {
                        damage_events.write(DamageEvent {
                            target: primary.entity,
                            amount: params.get("damage") * (bleed_crit_mult - 1.0),
                            source: trigger.owner,
                            tags: vec![DamageTag::Physical],
                        });
                    }
                    if let Ok(mut charges) = charges.get_mut(trigger.owner) {
                        if charges.spend_one() {
                            let cleave_radius = params.get("cleave_radius");
                            for t in targets {
                                if t.entity != primary.entity && t.pos.distance(primary.pos) <= cleave_radius {
                                    status_events.write(ApplyStatusEvent {
                                        target: t.entity,
                                        source: trigger.owner,
                                        effect_id: "bleed".to_string(),
                                        stacks: 1,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // Primal Pounce (Phase 9.4 — Druid): base damage + bleed to the leap target land via
            // `effects` above (unconditional — Primal Pounce isn't gated by Enhanced). "If the
            // target is rooted, deal triple damage" (`primal_pounce_root_triple_rare`) — a flat
            // top-up of +200% (base 100% + this 200% = 300%), same top-up shape as Ferocious Bite's
            // bleed crit above. "Create a Bloom flower at the point you jumped from"
            // (`primal_pounce_bloom_flower_rare`) — `outcome.origin` IS the pre-leap cast position
            // (behaviors never mutate it), so this is a direct reuse of Bloom's own pickup-spawn
            // primitive, no new mechanic.
            if instance.def_id == "primal_pounce" {
                if let Some(primary) = outcome.primary {
                    if active_hooks.map(|h| h.contains("primal_pounce_root_triple")).unwrap_or(false)
                        && has_status(primary.entity, "root", &marks)
                    {
                        damage_events.write(DamageEvent {
                            target: primary.entity,
                            amount: params.get("damage") * 2.0,
                            source: trigger.owner,
                            tags: vec![DamageTag::Physical],
                        });
                    }
                }
                if active_hooks.map(|h| h.contains("primal_pounce_bloom_flower")).unwrap_or(false) {
                    spawn_pickup(&mut commands, outcome.origin, PickUpKind::Enhance(1));
                }
            }

            // Flamewrath (Phase 9.5 — Mage): reuses `self_nova` as-is for "who's nearby" (its
            // `effects` list stays empty — the base behavior has no status awareness). This
            // special-case picks the NEAREST blaze-affected hit as the anchor, deals Fire damage to
            // everyone within `explosion_radius` of ITS position (not the caster's), and consumes
            // its blaze stack — unless the `flamewrath_no_consume` trade-off talent is active (its
            // own Pre hook already halved `damage` by the time this reads it).
            if instance.def_id == "flamewrath" {
                let ablaze_primary = outcome
                    .hits
                    .iter()
                    .filter(|h| has_status(h.entity, "blaze", &marks))
                    .min_by(|a, b| {
                        let da = a.pos.distance_squared(outcome.origin);
                        let db = b.pos.distance_squared(outcome.origin);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    });
                if let Some(primary) = ablaze_primary {
                    let explosion_radius = params.get("explosion_radius");
                    let damage = params.get("damage");
                    for t in targets {
                        if t.pos.distance(primary.pos) <= explosion_radius {
                            damage_events.write(DamageEvent {
                                target: t.entity,
                                amount: damage,
                                source: trigger.owner,
                                tags: vec![DamageTag::Fire],
                            });
                        }
                    }
                    let no_consume = active_hooks.map(|h| h.contains("flamewrath_no_consume")).unwrap_or(false);
                    if !no_consume {
                        remove_status_events.write(RemoveStatusEvent {
                            target: primary.entity,
                            effect_id: "blaze".to_string(),
                        });
                    }
                }
            }

            // Flamestrike (Phase 9.5 — Mage): base damage to every hit lands via `effects`
            // (AllHits) above. "Dealing increased damage per enemy affected by blaze" is base-kit
            // identity (not a talent), so it's this def_id-keyed top-up: for every blaze-affected
            // hit present, ALL hits take an extra `damage * bonus_per_blazed_percent / 100` Fire
            // damage — a global per-cast scaling the generic effects pipeline (one uniform amount
            // per `EffectSpec`) can't express, the same shape Consecrated Ground's own
            // count-scaling zone tick uses.
            if instance.def_id == "flamestrike" {
                let blazed_count = outcome.hits.iter().filter(|h| has_status(h.entity, "blaze", &marks)).count();
                if blazed_count > 0 {
                    let bonus = params.get("damage") * params.get("bonus_per_blazed_percent") / 100.0 * blazed_count as f32;
                    for hit in &outcome.hits {
                        damage_events.write(DamageEvent {
                            target: hit.entity,
                            amount: bonus,
                            source: trigger.owner,
                            tags: vec![DamageTag::Fire],
                        });
                    }
                }
            }

            cooldown.elapsed = 0.0;
            // Attack speed (Phase 9.1, §8.1(4)): effective_cd = resolved_cd / (1 + attack_speed).
            // attack_speed defaults to 0.0 (talent/modifier.rs's universal baseline) when no talent
            // grants it, so denom is 1.0 and this is identical to the old `resolved_cd` for every
            // shipped ability today. The `.max(0.05)` floor only guards a pathological >100%-per-
            // source haste stack from ever dividing by zero/negative — no such talent exists yet.
            // Always writing `duration` (removing the old `resolved_cd > 0.0` guard) also resolves
            // the §8.5 Override(0) debt: a talent that overrides an ability's cooldown to 0 now
            // actually takes effect, instead of silently leaving the previous duration in place.
            let resolved_cd = params.get("cooldown");
            let attack_speed = params.get("attack_speed");
            cooldown.duration = resolved_cd / (1.0 + attack_speed).max(0.05);
            break; // one instance per trigger
        }
    }
}

/// Whether `entity` currently carries the status effect `status_id` (Phase 9.3's holy-mark read
/// path, generalized in Phase 9.4 for Druid's root/bleed-conditional damage talents). A direct
/// query scan rather than a cached per-frame resource: status counts are small (a handful of
/// enemies at most) and this is only called from the targeted per-ability special-cases above.
fn has_status(entity: Entity, status_id: &str, statuses: &Query<&StatusEffectInstance>) -> bool {
    statuses.iter().any(|i| i.target == entity && i.def_id == status_id)
}

/// Builds a `PersistentZone` entity for a `dropped_zone` cast, from the ability's `zone` spec +
/// resolved params + the caster's faction. Occupant tick effects (Phase 6D) and projectile blocking
/// (Phase 6E) attach as extra components here as those steps land; a plain marker zone (Tree Conduit)
/// carries neither and is queried only via `PlayerZonePresence`.
fn spawn_dropped_zone(
    commands: &mut Commands,
    spec: &ZoneSpec,
    params: &ResolvedParams,
    center: Vec2,
    owner: Entity,
    faction: Faction,
) {
    let radius = params.get("zone_radius");
    let duration = params.get("zone_duration");
    // "follow_caster" (Phase 9.2 — AMZ's epic talent, Override(1.0)) forces Follow regardless of
    // the ability's own static `spec.anchor` — talents can't rewrite an AbilityDef field, only a
    // resolved param, so this is the escape hatch. Absent from every other zone ability's
    // base_params, so `params.get` defaults to 0.0 and they fall through to `spec.anchor` as before.
    let anchor = if params.get("follow_caster") > 0.5 {
        ZoneAnchor::Follow(owner)
    } else {
        match spec.anchor {
            ZoneAnchorKind::Fixed => ZoneAnchor::Fixed(center),
            ZoneAnchorKind::FollowCaster => ZoneAnchor::Follow(owner),
        }
    };
    let mut zone = commands.spawn((
        PersistentZone {
            zone_type: spec.zone_type.clone(),
            owner,
            radius,
            duration: Timer::from_seconds(duration, TimerMode::Once),
            anchor,
        },
        WorldPosition(center),
        faction,
    ));
    // Occupant tick effects (Phase 6D) — attached only when the ability defines any (Consecrated
    // Ground DoT, D&D regen). `regen_percent_per_second` is a percent of the owner's max health.
    // "slow_active"/"count_scaling_active" (Phase 9.3 — Consecrated Ground's talents): the same
    // resolved-param-flag escape hatch as "follow_caster" above — a talent can't rewrite the
    // ability's own static RON, only a param, so these default to 0.0 (absent) for every other
    // zone ability and cost nothing.
    let damage_per_second = params.get("damage_per_second");
    let regen_fraction = params.get("regen_percent_per_second") / 100.0;
    if damage_per_second > 0.0 || regen_fraction > 0.0 {
        let slow_status = (params.get("slow_active") > 0.5).then(|| "consecrated_slow".to_string());
        zone.insert(ZoneEffects {
            damage_per_second,
            regen_fraction,
            tick: Timer::from_seconds(ZONE_TICK_INTERVAL, TimerMode::Repeating),
            slow_status,
            scales_with_occupants: params.get("count_scaling_active") > 0.5,
        });
    }
    // AMZ (Phase 6E): destroys opposing-faction projectiles that enter it.
    if spec.blocks_projectiles {
        zone.insert(ZoneBlocksProjectiles);
    }
}

