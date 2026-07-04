// Phase 4: Hero / class identity and stance system.
//
// NOT yet wired into main.rs or GamePlugin. Add in Phase 4.
// Phase 1 uses a hardcoded stub (single default stance, DK abilities) so the ability
// system can be exercised before HeroDef assets are loaded.
//
// Module map:
//   assets.rs    — HeroDef RON asset (one file per class, assets/heroes/*.ron)
//   components.rs — HeroId, ActiveStance, ClassResource on the player entity
//   systems/
//     input_slot.rs — translates InputSlot + ActiveStance → AbilityId, emits TriggerAbility
//     stance.rs     — handles Q press: swaps ActiveStance, fires the stance-swap ability

pub mod assets;
pub mod components;
pub mod plugin;
pub mod systems;
