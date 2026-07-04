// Phase 1: Ability system — replaces hardcoded player_circle_attack / player_arc_attack.
//
// NOT yet wired into main.rs or GamePlugin. Add in Phase 1 of the migration plan.
//
// Module map:
//   assets.rs      — AbilityDef RON asset (loaded from assets/abilities/*.ron)
//   behavior.rs    — BehaviorRegistry resource + AbilityHook trait (the open extension point)
//   components.rs  — AbilityInstance component (per-unlocked-ability entity, child of player)
//   systems/
//     execute.rs       — drives ability cooldowns and calls BehaviorRegistry hooks
//     resolve_params.rs — applies the talent modifier stack to produce ResolvedParams

pub mod assets;
pub mod behavior;
pub mod components;
pub mod plugin;
pub mod systems;
