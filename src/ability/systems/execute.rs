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
use crate::ability::assets::{AbilityDef, AbilityLibrary, Activation, HookPhase, ZoneAnchorKind, ZoneSpec};
use crate::ability::behavior::{AbilityContext, BehaviorRegistry, ResolvedParams, Target, VfxShape};
use crate::ability::components::{AbilityCooldown, AbilityInstance, CastVfxEvent, CastVfxKind, TriggerAbilityEvent};
use crate::ability::effects::{apply_resolved_effects, resolve_effects};
use crate::ability::hooks::{HookContext, HookRegistry};
use crate::core::components::{AbilitiesSuppressed, Facing, Faction, ForcedImpulse, WorldPosition};
use crate::core::events::{DamageEvent, HealEvent};
use crate::projectile::components::{ArcHitbox, Lifetime, Projectile, ProjectileMotion, ProjectilePayload};
use crate::status::components::ApplyStatusEvent;
use crate::run::rng::RunRng;
use crate::run::state::RoomModifiers;
use crate::talent::assets::{StatModifier, TalentDef, TalentLibrary};
use crate::talent::components::{AcquiredTalents, ActiveHooks};
use crate::talent::modifier::resolve_params;
use crate::constants::ZONE_TICK_INTERVAL;
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
    // Grouped into one tuple SystemParam to stay under Bevy's 16-param-per-system limit.
    (registry, hook_registry, zone_presence, room_mods, mut rng): (
        Res<BehaviorRegistry>,
        Res<HookRegistry>,
        Res<PlayerZonePresence>,
        Res<RoomModifiers>,
        ResMut<RunRng>,
    ),
    library: Res<AbilityLibrary>,
    defs: Res<Assets<AbilityDef>>,
    talent_defs: Res<Assets<TalentDef>>,
    talent_library: Res<TalentLibrary>,
    owners: Query<(&WorldPosition, &Facing, &Faction, Option<&AcquiredTalents>, Option<&ActiveHooks>)>,
    // Candidate targets are actors only — never zones (which also carry WorldPosition + Faction, so
    // without this guard a friendly zone could be gathered/targeted by an enemy's cast).
    actors: Query<(Entity, &WorldPosition, &Faction), Without<PersistentZone>>,
    suppressed: Query<(), With<AbilitiesSuppressed>>,
    mut instances: Query<(&AbilityInstance, &mut AbilityCooldown)>,
) {
    // Gather candidate targets once per faction for all abilities fired this frame. A cast is
    // handed the list opposing its caster's faction (§Phase 5 faction-aware targeting).
    let mut friendly: Vec<Target> = Vec::new();
    let mut hostile: Vec<Target> = Vec::new();
    for (entity, pos, faction) in &actors {
        let t = Target { entity, pos: pos.0 };
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
        let Ok((owner_pos, owner_facing, owner_faction, acquired_opt, active_hooks)) = owners.get(trigger.owner) else {
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
                                &HookContext { caster: trigger.owner, zones: &zone_presence },
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

            // Forced-movement impulse (Phase 9.1 — the Movement-slot dash/blink). Applied directly
            // to the caster, not the world, unlike the zone/projectile spawns above.
            if let Some(spawn) = outcome.forced_impulse {
                commands.entity(trigger.owner).insert(ForcedImpulse {
                    velocity: spawn.velocity,
                    timer: Timer::from_seconds(spawn.duration, TimerMode::Once),
                });
            }

            // Travelling projectile: spawn it carrying the baked effects for on-impact delivery.
            if let Some(spawn) = outcome.projectile {
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
                    },
                    Lifetime { timer: Timer::from_seconds(spawn.lifetime, TimerMode::Once) },
                ));
            }

            // Post hooks (Phase 6 — after effects apply): react to the resolved cast outcome (e.g.
            // count kills for bone shield). No shipped Post hook is registered yet — bone_shield's
            // grant needs the deferred shield/absorb system (§8.1(5)) — so this is inert today but
            // the boundary is wired.
            if let Some(active) = active_hooks {
                for (phase, hook_id) in &def.hooks {
                    if *phase == HookPhase::Post && active.contains(hook_id) {
                        if let Some(hook) = hook_registry.get(hook_id) {
                            hook.post(
                                &HookContext { caster: trigger.owner, zones: &zone_presence },
                                &outcome,
                            );
                        }
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
    let anchor = match spec.anchor {
        ZoneAnchorKind::Fixed => ZoneAnchor::Fixed(center),
        ZoneAnchorKind::FollowCaster => ZoneAnchor::Follow(owner),
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
    let damage_per_second = params.get("damage_per_second");
    let regen_fraction = params.get("regen_percent_per_second") / 100.0;
    if damage_per_second > 0.0 || regen_fraction > 0.0 {
        zone.insert(ZoneEffects {
            damage_per_second,
            regen_fraction,
            tick: Timer::from_seconds(ZONE_TICK_INTERVAL, TimerMode::Repeating),
        });
    }
    // AMZ (Phase 6E): destroys opposing-faction projectiles that enter it.
    if spec.blocks_projectiles {
        zone.insert(ZoneBlocksProjectiles);
    }
}

