// Phase 3: Status effect system — bleed, blaze, frostbite, holy mark, root, stun.
//
// NOT yet wired into main.rs or GamePlugin. Add in Phase 3.
//
// Key invariant: adding a new status effect (or a new element that cancels an existing one)
// requires only a new RON file in assets/status_effects/. No Rust changes for purely
// data-driven effects. Behavior hooks (e.g. "on-tick deal damage") reuse the same hook mechanism
// the ability system introduces in Phase 2.
//
// Module map:
//   assets.rs    — StatusEffectDef RON asset
//   components.rs — ActiveStatusEffects, StatusEffectInstance (per-instance child entity)
//   systems/
//     tick.rs          — advances effect timers, emits DamageEvent for DoT effects
//     cross_interact.rs — listens to DamageEvent.tags; removes effects cancelled by element

pub mod assets;
pub mod components;
pub mod plugin;
pub mod systems;
