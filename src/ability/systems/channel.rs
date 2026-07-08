// Channel completion (Phase 9.3 — Flash of Light; later Druid Heal / Mage Frost Impale reuse the
// same `Channeling` component). Ticks every live channel; the frame `remaining` finishes, resolves
// the self-heal + its talent-gated extras (baked at cast start — see `Channeling`'s doc comment)
// and removes the component. Runs in CombatSet::Damage alongside `execute_ready_abilities`, so a
// same-frame completion's DamageEvent/HealEvent/GainShieldEvent resolve this frame like every
// other emitter.

use bevy::prelude::*;
use crate::ability::assets::EffectTarget;
use crate::ability::components::{Channeling, Minion, MinionOwner};
use crate::ability::effects::ResolvedEffect;
use crate::core::components::{Facing, Faction, Health, WorldPosition};
use crate::core::events::{DamageEvent, DamageTag, GainShieldEvent, HealEvent};
use crate::hero::components::Charges;
use crate::projectile::components::{Lifetime, Projectile, ProjectileMotion, ProjectilePayload};
use crate::status::components::StatusEffectInstance;
use crate::zone::components::PersistentZone;

/// Fixed radius the Flash of Light epic's consecrated-ground explosion reaches (Mechanics: "a
/// small radius around yourself").
const CONSECRATED_RADIATE_RADIUS: f32 = 60.0;

pub fn tick_channels(
    mut commands: Commands,
    time: Res<Time>,
    mut channels: Query<(Entity, &mut Channeling)>,
    healths: Query<&Health>,
    positions: Query<&WorldPosition>,
    factions: Query<&Faction>,
    facings: Query<&Facing>,
    actors: Query<(Entity, &WorldPosition, &Faction), Without<PersistentZone>>,
    statuses: Query<&StatusEffectInstance>,
    minions: Query<(Entity, &MinionOwner), With<Minion>>,
    mut charges: Query<&mut Charges>,
    mut damage_events: EventWriter<DamageEvent>,
    mut heal_events: EventWriter<HealEvent>,
    mut shield_events: EventWriter<GainShieldEvent>,
) {
    for (caster, mut channel) in &mut channels {
        channel.remaining.tick(time.delta());
        if !channel.remaining.finished() {
            continue;
        }

        let mut heal_amount = healths
            .get(caster)
            .map(|h| h.max * channel.heal_percent / 100.0)
            .unwrap_or(0.0);

        // Druid Heal's "heal X% more per bleeding enemy within range" (Phase 9.4) — counted at
        // COMPLETION time (the caster may have moved throughout the channel), same reasoning as
        // radiate's fresh-position read below.
        if channel.bleed_bonus_percent > 0.0 {
            if let Ok(caster_pos) = positions.get(caster) {
                let bleeding_count = statuses
                    .iter()
                    .filter(|s| s.def_id == "bleed")
                    .filter(|s| positions.get(s.target).map(|p| p.0.distance(caster_pos.0) <= channel.bleed_bonus_range).unwrap_or(false))
                    .count();
                heal_amount *= 1.0 + channel.bleed_bonus_percent / 100.0 * bleeding_count as f32;
            }
        }

        // Overheal -> shield, computed BEFORE the heal lands (apply_heal clamps to max, so the
        // spillover has to be read off the pre-heal current/max here, not derived after the fact).
        if channel.overheal_to_shield {
            if let Ok(health) = healths.get(caster) {
                let overheal = (health.current + heal_amount - health.max).max(0.0);
                if overheal > 0.0 {
                    shield_events.write(GainShieldEvent { target: caster, amount: overheal });
                }
            }
        }

        if heal_amount > 0.0 {
            heal_events.write(HealEvent { target: caster, amount: heal_amount });
            // Druid Heal's "your heal also heals your Ent" (Phase 9.4) — the same flat amount to
            // every minion this caster owns.
            if channel.heals_ents {
                for (minion, owner) in &minions {
                    if owner.0 == caster {
                        heal_events.write(HealEvent { target: minion, amount: heal_amount });
                    }
                }
            }
        }

        // Druid Heal's "your next attack in animal form is enhanced" (Phase 9.4).
        if channel.grants_enhanced_charge {
            if let Ok(mut charges) = charges.get_mut(caster) {
                charges.gain(1);
            }
        }

        if channel.radiate_percent > 0.0 {
            radiate(
                caster,
                heal_amount * channel.radiate_percent / 100.0,
                channel.radiate_radius,
                &positions,
                &factions,
                &actors,
                &mut damage_events,
            );
        }

        if channel.consecrated_radiate_damage > 0.0 {
            radiate(
                caster,
                channel.consecrated_radiate_damage,
                CONSECRATED_RADIATE_RADIUS,
                &positions,
                &factions,
                &actors,
                &mut damage_events,
            );
        }

        // Mage Frost Impale's icicle (Phase 9.5): consume every held frost Charge on completion,
        // scaling damage per charge, and fire a piercing Frost projectile toward the caster's
        // CURRENT aim — read fresh here (like `radiate`'s fresh position), since "channeled while
        // moving" lets the caster keep re-aiming throughout the cast.
        if channel.icicle_damage > 0.0 {
            if let (Ok(caster_pos), Ok(caster_faction)) = (positions.get(caster), factions.get(caster)) {
                let spent = charges.get_mut(caster).map(|mut c| c.spend_all()).unwrap_or(0);
                let damage = channel.icicle_damage * (1.0 + spent as f32 * channel.icicle_charge_damage_percent / 100.0);
                let aim = facings.get(caster).map(|f| f.0).unwrap_or(Vec2::ZERO);
                let dir = if aim.length_squared() >= 1e-6 { aim.normalize() } else { Vec2::X };
                commands.spawn((
                    Projectile,
                    WorldPosition(caster_pos.0),
                    ProjectileMotion {
                        velocity: dir * channel.icicle_speed,
                        radius: channel.icicle_radius,
                        pierce_remaining: channel.icicle_pierce,
                    },
                    ProjectilePayload {
                        source: caster,
                        target_faction: caster_faction.opposing(),
                        effects: vec![ResolvedEffect::Damage {
                            amount: damage,
                            tags: vec![DamageTag::Frost],
                            target: EffectTarget::AllHits,
                            crit_chance: channel.icicle_crit_chance,
                            crit_mult: channel.icicle_crit_mult,
                        }],
                        already_hit: Vec::new(),
                        grants_frost_charge_on_frostbitten: false,
                        explode_on_impact: None,
                    },
                    Lifetime { timer: Timer::from_seconds(channel.icicle_lifetime, TimerMode::Once) },
                ));
            }
        }

        commands.entity(caster).remove::<Channeling>();
    }
}

/// Deals `amount` Holy damage to every opposing-faction actor within `radius` of `caster`'s
/// CURRENT position — channel completion may be well after cast start since the caster can move
/// throughout ("channeled while moving"), so this reads position fresh, not the cast-time origin.
fn radiate(
    caster: Entity,
    amount: f32,
    radius: f32,
    positions: &Query<&WorldPosition>,
    factions: &Query<&Faction>,
    actors: &Query<(Entity, &WorldPosition, &Faction), Without<PersistentZone>>,
    damage_events: &mut EventWriter<DamageEvent>,
) {
    let (Ok(caster_pos), Ok(caster_faction)) = (positions.get(caster), factions.get(caster)) else {
        return;
    };
    let opposing = caster_faction.opposing();
    for (entity, pos, faction) in actors {
        if *faction == opposing && pos.0.distance(caster_pos.0) <= radius {
            damage_events.write(DamageEvent {
                target: entity,
                amount,
                source: caster,
                tags: vec![DamageTag::Holy],
            });
        }
    }
}
