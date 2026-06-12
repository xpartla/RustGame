use bevy::prelude::{Commands, Entity, EventWriter, Query, With};
use crate::core::components::WorldPosition;
use crate::core::events::HealEvent;
use crate::pickup::components::{PickUp, PickUpKind};
use crate::pickup::constants::PICKUP_RADIUS;
use crate::player::components::Player;

/// Player↔pickup overlap test (same proximity pattern as the attacks). On contact, applies the
/// pickup's effect by emitting the matching event, then despawns it. Runs in
/// `CombatSet::Damage` so a `HealEvent` resolves the same frame via `apply_heal`
/// (`CombatSet::Apply`).
pub fn collect_pickups(
    mut commands: Commands,
    mut heal_events: EventWriter<HealEvent>,
    player: Query<(Entity, &WorldPosition), With<Player>>,
    pickups: Query<(Entity, &WorldPosition, &PickUp)>,
) {
    let Ok((player_entity, player_pos)) = player.single() else {
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
        }

        commands.entity(pickup_entity).despawn();
    }
}
