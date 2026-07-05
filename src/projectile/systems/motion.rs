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
use crate::core::events::{DamageEvent, HealEvent};
use crate::projectile::components::{ProjectileMotion, ProjectilePayload};
use crate::status::components::ApplyStatusEvent;

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

            let hit = HitTarget { entity: target, pos: target_pos.0 };
            apply_resolved_effects(
                &mut damage_events,
                &mut heal_events,
                &mut status_events,
                payload.source,
                &[hit],
                Some(hit),
                &payload.effects,
            );
            payload.already_hit.push(target);

            if motion.pierce_remaining == 0 {
                commands.entity(proj_entity).try_despawn();
                break;
            }
            motion.pierce_remaining -= 1;
        }
    }
}
