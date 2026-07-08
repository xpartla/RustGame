use bevy::prelude::{Commands, Entity, EventWriter, Query, With};
use crate::core::components::WorldPosition;
use crate::core::events::HealEvent;
use crate::hero::components::Charges;
use crate::pickup::components::{PickUp, PickUpKind};
use crate::pickup::constants::PICKUP_RADIUS;
use crate::player::components::Player;
use crate::status::components::ApplyStatusEvent;
use crate::talent::components::ActiveHooks;

/// Player↔pickup overlap test (same proximity pattern as the attacks). On contact, applies the
/// pickup's effect by emitting the matching event, then despawns it. Runs in
/// `CombatSet::Damage` so a `HealEvent` resolves the same frame via `apply_heal`
/// (`CombatSet::Apply`).
pub fn collect_pickups(
    mut commands: Commands,
    mut heal_events: EventWriter<HealEvent>,
    mut status_events: EventWriter<ApplyStatusEvent>,
    mut player: Query<(Entity, &WorldPosition, Option<&mut Charges>, Option<&ActiveHooks>), With<Player>>,
    pickups: Query<(Entity, &WorldPosition, &PickUp)>,
) {
    let Ok((player_entity, player_pos, mut charges, active_hooks)) = player.single_mut() else {
        return;
    };

    for (pickup_entity, pickup_pos, pickup) in &pickups {
        if player_pos.0.distance(pickup_pos.0) > PICKUP_RADIUS {
            continue;
        }

        match pickup.kind {
            PickUpKind::Heal(amount) => {
                heal_events.write(HealEvent { target: player_entity, amount });
            }
            // Phase 9.4 — Bloom. A no-op for a non-Charges hero (Charges component absent).
            PickUpKind::Enhance(n) => {
                if let Some(charges) = charges.as_mut() {
                    charges.gain(n);
                }
                // "(common) You gain X% movement speed after pickup" (bloom_movespeed_common) — a
                // Behavior-hook talent flag, applied as a timed status like every other buff.
                if active_hooks.map(|h| h.contains("bloom_movespeed")).unwrap_or(false) {
                    status_events.write(ApplyStatusEvent {
                        target: player_entity,
                        source: player_entity,
                        effect_id: "bloom_swiftness".to_string(),
                        stacks: 1,
                    });
                }
            }
        }

        commands.entity(pickup_entity).despawn();
    }
}
