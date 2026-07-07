// Bridges the integer `Charges` bar (Phase 9.1) into the generic float `ClassResource` the HUD
// already renders (ui/screens/hud.rs::update_class_resource) — so a `Charges`-backed hero's class
// resource lights up with zero HUD work the moment its content grants/spends charges (Mage frost
// charges / Druid enhanced charges, Phase 9.4/9.5). Inert today: no shipped hero carries `Charges`.

use bevy::prelude::*;
use crate::hero::components::{Charges, ClassResource};

pub fn sync_charges_to_class_resource(
    mut commands: Commands,
    mut changed: Query<(Entity, &Charges, Option<&mut ClassResource>), Changed<Charges>>,
) {
    for (entity, charges, resource) in &mut changed {
        match resource {
            Some(mut r) => {
                r.current = charges.current as f32;
                r.max = charges.max as f32;
            }
            None => {
                commands.entity(entity).insert(ClassResource {
                    current: charges.current as f32,
                    max: charges.max as f32,
                });
            }
        }
    }
}
