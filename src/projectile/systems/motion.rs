// Travelling-projectile movement + collision (Phase 3D).
//
//   move_projectiles     — integrates each projectile's position by its velocity.
//   projectile_collision — on contact with an enemy (distance ≤ projectile radius + enemy radius),
//                          applies the projectile's baked effects via the shared applier, records
//                          the hit, and despawns once pierce is exhausted.
//
// Both run in CombatSet::Damage so the DamageEvents they emit resolve the same frame (mirroring
// the melee cone). Projectiles are spawned (deferred) by the ability execute system, so a shot
// begins moving the frame after its cast.

use bevy::prelude::*;
use crate::ability::effects::apply_resolved_effects;
use crate::ability::behavior::HitTarget;
use crate::core::components::{Faction, Hurtbox, WorldPosition};
use crate::core::events::{DamageEvent, DamageTag, HealEvent};
use crate::hero::components::Charges;
use crate::projectile::components::{ProjectileMotion, ProjectilePayload};
use crate::run::rng::RunRng;
use crate::status::components::{ApplyStatusEvent, StatusEffectInstance};

pub fn move_projectiles(time: Res<Time>, mut projectiles: Query<(&mut WorldPosition, &ProjectileMotion)>) {
    let dt = time.delta_secs();
    for (mut pos, motion) in &mut projectiles {
        pos.0 += motion.velocity * dt;
    }
}

pub fn projectile_collision(
    mut commands: Commands,
    mut damage_events: EventWriter<DamageEvent>,
    mut heal_events: EventWriter<HealEvent>,
    mut status_events: EventWriter<ApplyStatusEvent>,
    mut rng: ResMut<RunRng>,
    mut charges: Query<&mut Charges>,
    statuses: Query<&StatusEffectInstance>,
    mut projectiles: Query<(Entity, &WorldPosition, &mut ProjectileMotion, &mut ProjectilePayload)>,
    targets: Query<(Entity, &WorldPosition, &Hurtbox, &Faction)>,
) {
    for (proj_entity, proj_pos, mut motion, mut payload) in &mut projectiles {
        for (target, target_pos, hurtbox, faction) in &targets {
            // Only actors of the projectile's target faction can be hit (Phase 5): a player shot
            // strikes Hostiles, an enemy shot strikes the Friendly player.
            if *faction != payload.target_faction {
                continue;
            }
            if payload.already_hit.contains(&target) {
                continue;
            }
            if proj_pos.0.distance(target_pos.0) > motion.radius + hurtbox.radius {
                continue;
            }

            // Frostbolt's innate frost-charge generation (Phase 9.5): checked BEFORE this hit's own
            // effects apply (its `ApplyStatus(frostbite)` is only queued below, not yet a live
            // `StatusEffectInstance`), so this only fires when the target was ALREADY frostbitten by
            // a prior cast.
            if payload.grants_frost_charge_on_frostbitten
                && statuses.iter().any(|s| s.target == target && s.def_id == "frostbite")
            {
                if let Ok(mut charges) = charges.get_mut(payload.source) {
                    charges.gain(1);
                }
            }

            let hit = HitTarget { entity: target, pos: target_pos.0 };
            apply_resolved_effects(
                &mut damage_events,
                &mut heal_events,
                &mut status_events,
                &mut rng,
                payload.source,
                &[hit],
                Some(hit),
                &payload.effects,
            );

            // Fireblast's "explodes on impact" unique talent (Phase 9.5): extra Fire damage to
            // every OTHER opposing-faction actor within `radius` of the impact point.
            if let Some((explode_damage, explode_radius)) = payload.explode_on_impact {
                for (other, other_pos, _, other_faction) in &targets {
                    if other != target
                        && *other_faction == payload.target_faction
                        && other_pos.0.distance(target_pos.0) <= explode_radius
                    {
                        damage_events.write(DamageEvent {
                            target: other,
                            amount: explode_damage,
                            source: payload.source,
                            tags: vec![DamageTag::Fire],
                        });
                    }
                }
            }

            payload.already_hit.push(target);

            if motion.pierce_remaining == 0 {
                commands.entity(proj_entity).try_despawn();
                break;
            }
            motion.pierce_remaining -= 1;
        }
    }
}
