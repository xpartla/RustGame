// Minion lifecycle (Phase 9.2 — Companion's `summon` behavior).
//
// A minion is a genuinely independent caster: it carries its own Faction/WorldPosition/Facing and
// an AbilityInstance (its own attack ability, e.g. "companion_attack"), so the existing
// faction-aware ability engine (execute_ready_abilities) fires its attacks with zero changes —
// the engine was already agnostic to "is this caster the player." This module only handles what's
// specific to a minion: seeking a target to fight and expiring.
//
// Deliberately NOT reusing the enemy flow-field follower (as originally sketched): FlowField is a
// single shared BFS built from the PLAYER's position outward (for enemies to path toward the
// player) — exactly backwards for a minion that needs to chase HOSTILES. `minion_seek_and_face`
// instead steers straight-line toward the nearest Hostile actor (no wall-avoidance); a reasonable
// simplification for a short-lived (single-digit seconds) pet, not a general pathing system.

use bevy::prelude::*;
use crate::ability::components::{AbilityInstance, Minion, MinionLifetime};
use crate::core::components::{Facing, Faction, Health, MoveSpeed, Velocity, WorldPosition};
use crate::constants::{MINION_ENGAGE_RANGE, MINION_SEEK_RANGE};
use crate::zone::components::PersistentZone;

/// Steers each minion toward the nearest Hostile actor within `MINION_SEEK_RANGE` (straight line)
/// and faces it (so its melee/aimed attack can fire); idles (zero velocity, last facing held) if
/// none is in range. Runs in `MovementSet::Intent`, mirroring `enemy_follow_flow_field`/
/// `ranged_caster_ai`'s role for enemies. `Without<PersistentZone>` mirrors
/// `execute_ready_abilities`' own target-gathering guard — a zone entity also carries
/// `WorldPosition`/`Faction` and must never be mistaken for an actor to chase.
///
/// Holds position once within `MINION_ENGAGE_RANGE`: without a stopping distance, a fast (45 u/s)
/// minion closing on a target already at melee range overshoots every frame, oscillating back and
/// forth across it — and because `execute_ready_abilities` reads `Facing` (set here, pre-move)
/// against the *post-move* position that same frame, an oscillating minion's facing keeps landing
/// on the wrong side of its target right when `companion_attack`'s cone check runs, whiffing
/// indefinitely. Holding still once close keeps facing and position mutually consistent.
pub fn minion_seek_and_face(
    hostiles: Query<(&WorldPosition, &Faction), Without<PersistentZone>>,
    mut minions: Query<(&WorldPosition, &mut Velocity, &mut Facing, &MoveSpeed), With<Minion>>,
) {
    for (pos, mut vel, mut facing, speed) in &mut minions {
        let nearest = hostiles
            .iter()
            .filter(|(_, f)| **f == Faction::Hostile)
            .map(|(p, _)| p.0)
            .filter(|p| p.distance(pos.0) <= MINION_SEEK_RANGE)
            .min_by(|a, b| a.distance(pos.0).partial_cmp(&b.distance(pos.0)).unwrap());

        match nearest {
            Some(target) => {
                let to_target = target - pos.0;
                let dist = to_target.length();
                if dist > 1e-6 {
                    let dir = to_target / dist;
                    facing.0 = dir;
                    vel.0 = if dist > MINION_ENGAGE_RANGE { dir * speed.0 } else { Vec2::ZERO };
                } else {
                    vel.0 = Vec2::ZERO;
                }
            }
            None => vel.0 = Vec2::ZERO,
        }
    }
}

/// Ticks each minion's remaining lifetime and reaps it (+ its owned `AbilityInstance`) on expiry
/// or death (`Health.current <= 0.0` — a minion has no XP/drop-table, unlike `enemy_death`).
pub fn update_minion_lifecycle(
    mut commands: Commands,
    time: Res<Time>,
    mut minions: Query<(Entity, &mut MinionLifetime, &Health), With<Minion>>,
    instances: Query<(Entity, &AbilityInstance)>,
) {
    let dt = time.delta();
    for (entity, mut lifetime, health) in &mut minions {
        lifetime.0.tick(dt);
        if !lifetime.0.finished() && health.current > 0.0 {
            continue;
        }
        for (instance_entity, instance) in &instances {
            if instance.owner == entity {
                commands.entity(instance_entity).despawn();
            }
        }
        commands.entity(entity).despawn();
    }
}
