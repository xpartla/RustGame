use bevy::log::info;
use bevy::prelude::{EventReader, EventWriter, Query};
use crate::core::events::{GainXpEvent, LevelUpEvent};
use crate::player::components::Experience;

/// The single point that mutates `Experience`. Drains `GainXpEvent`s, adds XP to the target's
/// progression, and rolls over as many levels as the gain covers (overflow carries into the
/// next level), emitting a `LevelUpEvent` for each. Entities without an `Experience` component
/// are silently skipped.
pub fn gain_experience(
    mut xp_events: EventReader<GainXpEvent>,
    mut level_events: EventWriter<LevelUpEvent>,
    mut query: Query<&mut Experience>,
) {
    for event in xp_events.read() {
        let Ok(mut exp) = query.get_mut(event.target) else {
            continue;
        };

        exp.current += event.amount;
        while exp.current >= exp.to_next {
            exp.current -= exp.to_next;
            exp.level += 1;
            exp.to_next = Experience::to_next_for(exp.level);
            level_events.write(LevelUpEvent { level: exp.level });
        }
    }
}

/// Placeholder level-up reward: just log it. This is the hook the talent system will replace.
pub fn apply_level_up_reward(
    mut level_events: EventReader<LevelUpEvent>,
) {
    for event in level_events.read() {
        info!("Level up! Now level {}.", event.level);
    }
}
