// Phase 1: Ability system — replaces hardcoded player_circle_attack / player_arc_attack.
//
// Wired into GamePlugin as AbilityPlugin (Phase 1 complete).
//
// Module map:
//   assets.rs      — AbilityDef RON asset (loaded from assets/abilities/*.ability.ron)
//   behavior.rs    — BehaviorRegistry resource + AbilityBehavior trait (the open extension point).
//                    The talent-driven HookRegistry arrives with the talent system in Phase 2.
//   components.rs  — AbilityInstance component (per-unlocked-ability entity, child of player)
//   systems/
//     execute.rs       — drives ability cooldowns and calls the registered behavior
//     resolve_params.rs — applies the talent modifier stack to produce ResolvedParams

pub mod assets;
pub mod behavior;
pub mod components;
pub mod effects;
pub mod hooks;
pub mod plugin;
pub mod systems;

pub use plugin::AbilityPlugin;
