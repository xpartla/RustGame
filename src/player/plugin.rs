use bevy::prelude::*;
use crate::core::sets::CombatSet;
use crate::game::state::GameState;
use crate::player::systems::input::player_input;
use crate::player::systems::spawn_player::spawn_player;
use crate::player::systems::ability_input::player_ability_input;
use crate::player::systems::update_player_facing::update_player_facing;
use crate::player::systems::death::player_death;
use crate::player::systems::experience::{apply_level_up_reward, gain_experience};

// Presentation note: draw_player_facing and the player's visuals (attach_player_visuals) are
// registered by game::presentation::PresentationPlugin; spawn_player creates logic
// components only.

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player);
        app.add_systems(
            Update,
            (
                player_input,
                update_player_facing.before(CombatSet::Damage),
                // Input → ability trigger. Fires before CombatSet::Damage so the ability
                // system (also in that set) can act on the event the same frame.
                player_ability_input.after(update_player_facing).before(CombatSet::Damage),
                player_death.in_set(CombatSet::Death),
                // XP lands the same frame as a kill: enemy_death emits GainXpEvent in
                // CombatSet::Death, so consume it after that set runs.
                gain_experience.after(CombatSet::Death),
                apply_level_up_reward.after(gain_experience),
            ).run_if(in_state(GameState::InRun)),
        );
    }
}
