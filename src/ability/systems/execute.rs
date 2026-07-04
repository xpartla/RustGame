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
use crate::ability::assets::{AbilityDef, AbilityLibrary};
use crate::ability::behavior::{AbilityContext, AbilityEffect, BehaviorRegistry, EnemyTarget};
use crate::ability::components::{AbilityCooldown, AbilityInstance, TriggerAbilityEvent};
use crate::ability::systems::resolve_params::resolve_params;
use crate::core::components::{Facing, WorldPosition};
use crate::core::events::{DamageEvent, HealEvent};
use crate::enemy::components::Enemy;
use crate::projectile::components::{ArcHitbox, Lifetime, Projectile};

/// Advances every ability's cooldown timer toward readiness.
pub fn tick_ability_cooldowns(time: Res<Time>, mut cooldowns: Query<&mut AbilityCooldown>) {
    let dt = time.delta_secs();
    for mut cooldown in &mut cooldowns {
        if cooldown.elapsed < cooldown.duration {
            cooldown.elapsed += dt;
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
    registry: Res<BehaviorRegistry>,
    library: Res<AbilityLibrary>,
    defs: Res<Assets<AbilityDef>>,
    owners: Query<(&WorldPosition, &Facing)>,
    enemies: Query<(Entity, &WorldPosition), With<Enemy>>,
    mut instances: Query<(&AbilityInstance, &mut AbilityCooldown)>,
) {
    // Gather candidate targets once for all abilities fired this frame.
    let targets: Vec<EnemyTarget> = enemies
        .iter()
        .map(|(entity, pos)| EnemyTarget { entity, pos: pos.0 })
        .collect();

    for trigger in triggers.read() {
        let Ok((owner_pos, owner_facing)) = owners.get(trigger.owner) else {
            continue;
        };
        // No aim direction yet (Facing starts at zero until the first mouse move).
        if owner_facing.0.length_squared() < 1e-6 {
            continue;
        }

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

            let params = resolve_params(&def.base_params);
            let ctx = AbilityContext {
                owner: trigger.owner,
                origin: owner_pos.0,
                facing: owner_facing.0.normalize(),
                enemies: &targets,
            };
            let mut effects = Vec::new();
            behavior.execute(&ctx, &params, &mut effects);
            apply_effects(&mut commands, &mut damage_events, &mut heal_events, trigger.owner, effects);

            cooldown.elapsed = 0.0;
            let resolved_cd = params.get("cooldown");
            if resolved_cd > 0.0 {
                cooldown.duration = resolved_cd;
            }
            break; // one instance per trigger
        }
    }
}

/// Applies the effects a behavior produced. The only place ability execution touches the world.
fn apply_effects(
    commands: &mut Commands,
    damage_events: &mut EventWriter<DamageEvent>,
    heal_events: &mut EventWriter<HealEvent>,
    source: Entity,
    effects: Vec<AbilityEffect>,
) {
    for effect in effects {
        match effect {
            AbilityEffect::Damage { target, amount, tags } => {
                damage_events.write(DamageEvent { target, amount, source, tags });
            }
            AbilityEffect::Heal { target, amount } => {
                heal_events.write(HealEvent { target, amount });
            }
            AbilityEffect::ConeVfx { origin, radius, half_angle, forward, lifetime } => {
                commands.spawn((
                    Projectile,
                    WorldPosition(origin),
                    ArcHitbox { radius, half_angle },
                    Facing(forward),
                    Lifetime { timer: Timer::from_seconds(lifetime, TimerMode::Once) },
                ));
            }
        }
    }
}
