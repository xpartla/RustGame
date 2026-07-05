// resolve_actor_status — folds each actor's active status instances into the generic modifier
// components that movement and damage read (Phase 3C).
//
//   move_speed_mult (product) → MoveSpeedModifier   (frostbite 0.8)
//   damage_taken_mult (product) → DamageTakenModifier (frostbite 1.1)
//   any immobilize            → Immobilized marker    (root, stun)
//
// Runs last in StatusSet::CrossInteract, so it reflects this frame's applications AND removals.
// The resolved components are consumed by next frame's apply_velocity / apply_damage — a uniform
// one-frame latency (docs/phase3-plan.md §2.6). Components are only inserted when a value actually
// deviates from neutral and removed when it returns to neutral, so status-free actors (and the
// whole physical-only campaign) never carry them — keeping the golden baseline unperturbed.

use bevy::prelude::*;
use std::collections::HashMap;
use crate::core::components::{AbilitiesSuppressed, DamageTakenModifier, Health, Immobilized, MoveSpeedModifier};
use crate::status::assets::{StatusEffectDef, StatusLibrary};
use crate::status::components::StatusEffectInstance;

fn is_neutral(x: f32) -> bool {
    (x - 1.0).abs() < 1e-6
}

pub fn resolve_actor_status(
    mut commands: Commands,
    instances: Query<&StatusEffectInstance>,
    library: Res<StatusLibrary>,
    defs: Res<Assets<StatusEffectDef>>,
    mut actors: Query<
        (
            Entity,
            Option<&mut MoveSpeedModifier>,
            Option<&mut DamageTakenModifier>,
            Option<&Immobilized>,
            Option<&AbilitiesSuppressed>,
        ),
        With<Health>,
    >,
) {
    // Net modifiers per target from active instances: (move×, damage×, immobilize, suppress).
    let mut acc: HashMap<Entity, (f32, f32, bool, bool)> = HashMap::new();
    for inst in &instances {
        let Some(def) = library.get(&inst.def_id).and_then(|h| defs.get(h)) else {
            continue;
        };
        let e = acc.entry(inst.target).or_insert((1.0, 1.0, false, false));
        e.0 *= def.move_speed_mult;
        e.1 *= def.damage_taken_mult;
        e.2 |= def.immobilize;
        e.3 |= def.suppress_abilities;
    }

    for (entity, move_mod, dmg_mod, immobilized, suppressed) in &mut actors {
        let (move_mult, dmg_mult, immobile, suppress) =
            acc.get(&entity).copied().unwrap_or((1.0, 1.0, false, false));

        // MoveSpeedModifier: update in place, insert when it first deviates, drop when neutral.
        match move_mod {
            Some(mut mm) => {
                if is_neutral(move_mult) {
                    commands.entity(entity).remove::<MoveSpeedModifier>();
                } else {
                    mm.0 = move_mult;
                }
            }
            None => {
                if !is_neutral(move_mult) {
                    commands.entity(entity).insert(MoveSpeedModifier(move_mult));
                }
            }
        }

        // DamageTakenModifier: same reconciliation.
        match dmg_mod {
            Some(mut dm) => {
                if is_neutral(dmg_mult) {
                    commands.entity(entity).remove::<DamageTakenModifier>();
                } else {
                    dm.0 = dmg_mult;
                }
            }
            None => {
                if !is_neutral(dmg_mult) {
                    commands.entity(entity).insert(DamageTakenModifier(dmg_mult));
                }
            }
        }

        // Immobilized marker.
        match (immobile, immobilized.is_some()) {
            (true, false) => {
                commands.entity(entity).insert(Immobilized);
            }
            (false, true) => {
                commands.entity(entity).remove::<Immobilized>();
            }
            _ => {}
        }

        // AbilitiesSuppressed marker (stun): same insert-when-active / remove-when-clear reconcile.
        match (suppress, suppressed.is_some()) {
            (true, false) => {
                commands.entity(entity).insert(AbilitiesSuppressed);
            }
            (false, true) => {
                commands.entity(entity).remove::<AbilitiesSuppressed>();
            }
            _ => {}
        }
    }
}
