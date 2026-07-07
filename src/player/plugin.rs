use bevy::prelude::*;
use crate::core::sets::{CombatSet, MovementSet};
use crate::game::state::GameState;
use crate::player::components::Player;
use crate::player::systems::base_stats::apply_base_stats;
use crate::player::systems::input::player_input;
use crate::player::systems::spawn_player::spawn_player;
use crate::player::systems::update_player_facing::update_player_facing;
use crate::player::systems::death::player_death;
use crate::player::systems::experience::{apply_level_up_reward, gain_experience};

// Presentation note: draw_player_facing and the player's visuals (attach_player_visuals) are
// registered by game::presentation::PresentationPlugin; spawn_player creates logic
// components only.

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // Phase 8, §5: moved from Startup to OnEnter(InRun) — the "seed the initial world" moment
        // for a game that (windowed) now sits in Login/Menu at boot with no live player underneath.
        // `OnEnter(InRun)` still fires once unconditionally at the very start (GameState::InRun is
        // the app's default state), so this still seeds the world exactly like the old Startup
        // registration did — it just also fires on every *later* re-entry into InRun (every overlay
        // round-trip, every real run-start/restart/resume). The guard makes those re-entries inert:
        // by the time any of them applies, a Player already exists (spawned by this same system at
        // boot, or freshly respawned by reset_and_start_run/resume_run before they set the state).
        app.add_systems(
            OnEnter(GameState::InRun),
            spawn_player.run_if(not(any_with_component::<Player>)),
        );
        // Deferred, ungated base_stats application (Phase 9.2) — mirrors
        // ability/plugin.rs::grant_level_1_abilities's HeroDef-load deferral.
        app.add_systems(Update, apply_base_stats);
        app.add_systems(
            Update,
            (
                player_input.in_set(MovementSet::Intent),
                update_player_facing.before(CombatSet::Damage),
                // Input → ability trigger now lives in the hero indirection layer
                // (hero/systems/input_slot.rs, wired by HeroPlugin), which reads ActiveStance.
                player_death.in_set(CombatSet::Death),
                // XP lands the same frame as a kill: enemy_death emits GainXpEvent in
                // CombatSet::Death, so consume it after that set runs.
                gain_experience.after(CombatSet::Death),
                apply_level_up_reward.after(gain_experience),
            ).run_if(in_state(GameState::InRun)),
        );
    }
}
