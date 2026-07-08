// Ent taunt (Phase 9.4 — Druid's Spawn Ent): "runs towards the nearest enemy, forcing the enemy to
// attack the Ent instead of you." A `Taunt`-carrying Friendly minion (the Ent) pulls any Hostile
// `MeleeChaser` within its radius off the player's flow field and onto a straight-line chase of the
// Ent itself — mirroring `ability::systems::summon::minion_seek_and_face`'s own straight-line
// steering for the same reason (the shared `FlowField` only ever points toward the player).
//
// Scoped to `MeleeChaser` only: `RangedCaster`/`Stationary` AI drive their own movement (approach-
// and-hold / stand-still) and, since `contact_melee`/most enemy abilities already hit ANY
// opposing-faction target within range (not just the player), an enemy standing next to a taunting
// Ent can already damage it with zero further changes — only the STEERING needed a new path.

use bevy::prelude::*;
use crate::core::components::{Faction, WorldPosition};
use crate::enemy::components::{AiBehavior, Enemy, Taunt, Taunted};

/// Runs in `MovementSet::Intent`, before `enemy_follow_flow_field` so the same frame's steering
/// already sees a fresh `Taunted` marker. Recomputed from scratch every frame (cheap — pack sizes
/// are small): an enemy that leaves every Ent's radius has its `Taunted` marker removed the same
/// frame, resuming normal flow-field chase.
pub fn apply_ent_taunt(
    mut commands: Commands,
    taunters: Query<(Entity, &WorldPosition, &Taunt, &Faction)>,
    mut enemies: Query<(Entity, &WorldPosition, Option<&Taunted>), (With<Enemy>, With<AiBehavior>)>,
) {
    for (enemy, pos, existing) in &mut enemies {
        let nearest = taunters
            .iter()
            .filter(|(_, t_pos, taunt, faction)| {
                **faction == Faction::Friendly && pos.0.distance(t_pos.0) <= taunt.radius
            })
            .min_by(|(_, a_pos, ..), (_, b_pos, ..)| {
                a_pos.0.distance(pos.0).partial_cmp(&b_pos.0.distance(pos.0)).unwrap()
            })
            .map(|(taunter, ..)| taunter);

        match (nearest, existing) {
            (Some(taunter), Some(current)) if current.0 == taunter => {} // unchanged
            (Some(taunter), _) => {
                commands.entity(enemy).insert(Taunted(taunter));
            }
            (None, Some(_)) => {
                commands.entity(enemy).remove::<Taunted>();
            }
            (None, None) => {}
        }
    }
}
