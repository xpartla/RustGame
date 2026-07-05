// Drives the per-frame ability execution loop.
//
// Two systems (chained, in CombatSet::Damage so emitted DamageEvents resolve this frame):
//   tick_ability_cooldowns  — advances AbilityCooldown.elapsed for every AbilityInstance
//   execute_ready_abilities — for each TriggerAbilityEvent, fires the matching ready ability
//
// Per fire:
//   1. resolve_params(base_params)                      → ResolvedParams
//   2. BehaviorRegistry.get(behavior_id).execute(...)   → pushes AbilityEffects
//   3. apply_effects(...)                               → DamageEvent / HealEvent / VFX
//   4. reset cooldown (duration taken from params("cooldown"))
//
// Hooks (AbilityDef.hooks) are not run in Phase 1 — they arrive with the talent system.

use bevy::prelude::*;
use crate::ability::assets::{AbilityDef, AbilityLibrary, Activation};
use crate::ability::behavior::{AbilityContext, BehaviorRegistry, EnemyTarget, VfxShape};
use crate::ability::components::{AbilityCooldown, AbilityInstance, TriggerAbilityEvent};
use crate::ability::effects::{apply_resolved_effects, resolve_effects};
use crate::core::components::{Facing, WorldPosition};
use crate::core::events::{DamageEvent, HealEvent};
use crate::enemy::components::Enemy;
use crate::projectile::components::{ArcHitbox, Lifetime, Projectile, ProjectileMotion, ProjectilePayload};
use crate::status::components::ApplyStatusEvent;
use crate::talent::assets::{TalentDef, TalentLibrary};
use crate::talent::components::AcquiredTalents;
use crate::talent::modifier::resolve_params;

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
    instances: Query<(&AbilityInstance, &AbilityCooldown)>,
) {
    for (instance, cooldown) in &instances {
        if !cooldown.is_ready() {
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
    registry: Res<BehaviorRegistry>,
    library: Res<AbilityLibrary>,
    defs: Res<Assets<AbilityDef>>,
    talent_defs: Res<Assets<TalentDef>>,
    talent_library: Res<TalentLibrary>,
    owners: Query<(&WorldPosition, &Facing, Option<&AcquiredTalents>)>,
    enemies: Query<(Entity, &WorldPosition), With<Enemy>>,
    mut instances: Query<(&AbilityInstance, &mut AbilityCooldown)>,
) {
    // Gather candidate targets once for all abilities fired this frame.
    let targets: Vec<EnemyTarget> = enemies
        .iter()
        .map(|(entity, pos)| EnemyTarget { entity, pos: pos.0 })
        .collect();

    // Fallback for owners without a talent list (e.g. non-player casters) — an empty stack.
    let no_talents = AcquiredTalents::default();

    for trigger in triggers.read() {
        let Ok((owner_pos, owner_facing, acquired_opt)) = owners.get(trigger.owner) else {
            continue;
        };
        let acquired = acquired_opt.unwrap_or(&no_talents);
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

            let params = resolve_params(
                &instance.def_id,
                &def.base_params,
                acquired,
                &talent_defs,
                &talent_library,
                &[],
            );
            let ctx = AbilityContext {
                owner: trigger.owner,
                origin: owner_pos.0,
                // Non-zero for needs-aim casts (gated above); zero is fine for self-centred shapes.
                facing: owner_facing.0.normalize_or_zero(),
                enemies: &targets,
            };
            let outcome = behavior.resolve(&ctx, &params);
            let resolved = resolve_effects(&def.effects, &params);
            // Instant hits (cone/nova). Empty for a pure projectile cast (delivery is deferred).
            apply_resolved_effects(
                &mut damage_events,
                &mut heal_events,
                &mut status_events,
                trigger.owner,
                &outcome.hits,
                outcome.primary,
                &resolved,
            );

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
                        effects: resolved.clone(),
                        already_hit: Vec::new(),
                    },
                    Lifetime { timer: Timer::from_seconds(spawn.lifetime, TimerMode::Once) },
                ));
            }

            cooldown.elapsed = 0.0;
            let resolved_cd = params.get("cooldown");
            if resolved_cd > 0.0 {
                cooldown.duration = resolved_cd;
            }
            break; // one instance per trigger
        }
    }
}

