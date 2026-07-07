use bevy::app::{App, First};
use bevy::ecs::event::Events;
use bevy::prelude::{Entity, Event, IntoScheduleConfigs, OnEnter, ResMut, in_state};
use crate::game::state::GameState;

/// Registers a gameplay event whose pending instances survive overlay states (Phase 3.1).
///
/// `App::add_event` expires unread events after two frames, unconditionally. Every combat
/// event reader is gated on `GameState::InRun`, so an event written on the frame an overlay
/// (TalentPicker, Paused, Merchant) opens could expire before its reader ran again — e.g. a
/// DoT tick's DamageEvent is written in StatusSet::Tick but consumed by `apply_damage` only on
/// the NEXT frame, and was silently lost whenever a level-up opened the picker in between.
///
/// Events registered here advance their buffers only during InRun frames: the world freezes
/// with pending events intact, and they resolve on the first frame after resume. (The `First`-
/// schedule run condition sees the pre-transition state, so the open frame gets one final
/// buffer advance — events written that frame stay readable — and the resume frame skips one;
/// both are harmless because readers always run before the following advance.)
///
/// Entering a terminal state (GameOver, Menu) clears pending events so a dead run's combat
/// never leaks into the next run.
///
/// Rule of thumb: events that RESOLVE combat outcomes (damage, heal, status) register here;
/// events expressing input intent (TriggerAbilityEvent) or consumed in the same frame they are
/// written (GainXpEvent, LevelUpEvent, UnlockAbilityEvent) use plain `add_event`.
pub trait AddGameplayEventExt {
    fn add_gameplay_event<T: Event>(&mut self) -> &mut Self;
}

impl AddGameplayEventExt for App {
    fn add_gameplay_event<T: Event>(&mut self) -> &mut Self {
        self.init_resource::<Events<T>>()
            .add_systems(First, update_events::<T>.run_if(in_state(GameState::InRun)))
            .add_systems(OnEnter(GameState::GameOver), clear_events::<T>)
            .add_systems(OnEnter(GameState::Menu), clear_events::<T>)
    }
}

fn update_events<T: Event>(mut events: ResMut<Events<T>>) {
    events.update();
}

fn clear_events<T: Event>(mut events: ResMut<Events<T>>) {
    events.clear();
}

/// Element tags on a `DamageEvent`. Used by the status effect system (Phase 3) to
/// trigger cross-element cancellations (Fire removes Frostbite, Frost removes Blaze).
/// Existing callers pass an empty `tags` slice — the field is purely additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum DamageTag {
    Physical,
    Fire,
    Frost,
    Holy,
    Shadow,
    Arcane,
}

/// Request to deal `amount` damage to `target`. Any system (attacks, projectile
/// collisions, hazards, DoTs) emits this; `apply_damage` is the single place that
/// mutates `Health`. `source` records who caused it (for future attribution: reflect,
/// thorns, kill credit / XP).
///
/// `tags` — added in Phase 0. All existing callers pass an empty slice; the field is
/// read only by status/systems/cross_interact.rs (Phase 3).
#[derive(Event)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
    pub source: Entity,
    pub tags: Vec<DamageTag>,
}

/// Request to restore `amount` health to `target`. The healing counterpart to `DamageEvent`:
/// any system (pickups, regen, abilities) emits this; `apply_heal` is the single place that
/// adds to `Health`, clamping to `Health.max`.
#[derive(Event)]
pub struct HealEvent {
    pub target: Entity,
    pub amount: f32,
}

/// Request to award `amount` experience to `target`. Emitted on a kill (`enemy_death`, crediting
/// the killer via `LastHitBy`); `gain_experience` is the single place that mutates `Experience`.
/// `target`-based for future-proofing — only entities with an `Experience` component (the player)
/// actually gain XP; for anyone else it's a no-op.
#[derive(Event)]
pub struct GainXpEvent {
    pub target: Entity,
    pub amount: u32,
}

/// Fired by `gain_experience` each time the player crosses a level threshold. The hook for
/// level-up rewards (currently log-only; later the talent system).
#[derive(Event)]
pub struct LevelUpEvent {
    pub level: u32,
}

/// Request to grant `amount` of absorb shield to `target` (Phase 9.1, §8.1(5)). Additive: multiple
/// grants stack into the one `Absorb` component per entity (core/components.rs). Consumed
/// exclusively by `apply_shield_gain` (core/systems/apply_shield.rs). A combat-resolution outcome
/// like `DamageEvent`/`HealEvent`, so it is registered via `add_gameplay_event`.
#[derive(Event)]
pub struct GainShieldEvent {
    pub target: Entity,
    pub amount: f32,
}
