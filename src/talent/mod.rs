// Phase 2: Talent system — offer generation, uniqueness, modifier stack, merchant ops.
//
// NOT yet wired into main.rs or GamePlugin. Add in Phase 2.
//
// Module map:
//   assets.rs    — TalentDef RON asset
//   components.rs — AcquiredTalents + ActiveHooks on the player entity
//   modifier.rs  — StatModifier, ModOp, resolve_params() pure function
//   offer.rs     — TalentOffer, offer generation, uniqueness checks
//   systems/
//     apply.rs    — installs / removes ActiveHook components when talents are gained / removed
//     merchant.rs — remove-talent and 3-for-1 trade-up logic

pub mod assets;
pub mod components;
pub mod modifier;
pub mod offer;
pub mod plugin;
pub mod systems;
