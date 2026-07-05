// Phase 4: Hero / class identity and stance system. Wired into GameLogicPlugin via HeroPlugin.
//
// The hero indirection replaces the Phase-1 hardcoded LMB → death_strike stub: HeroDef defines
// each class's level-1 abilities, band pools, and per-stance InputSlot → AbilityId bindings; the
// player carries HeroIdentity + ActiveStance; HeroPlugin resolves input through them.
//
// Module map:
//   assets.rs     — HeroDef RON asset (one file per class, assets/heroes/*.hero.ron) + HeroLibrary
//   components.rs — HeroIdentity, ActiveStance, InputSlot, ClassResource (on the player entity)
//   systems/
//     input_slot.rs — translates InputSlot + ActiveStance → AbilityId, emits TriggerAbilityEvent
//     stance.rs     — handles Q press: swaps ActiveStance, applies the entered stance's swap effect

pub mod assets;
pub mod components;
pub mod plugin;
pub mod systems;

pub use plugin::HeroPlugin;
