use bevy::prelude::*;
use crate::core::sets::CombatSet;
use crate::game::state::GameState;
use crate::player::systems::input::player_input;
use crate::player::systems::spawn_player::spawn_player;
use crate::player::systems::attack::{player_arc_attack, player_circle_attack};
use crate::player::systems::update_player_facing::update_player_facing;
use crate::player::systems::debug::draw_player_facing;
use crate::player::systems::death::player_death;
use crate::player::systems::experience::{apply_level_up_reward, gain_experience};
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player);
        app.add_systems(
            Update,
            (
                player_input,
                update_player_facing,
                player_circle_attack.after(update_player_facing).in_set(CombatSet::Damage),
                player_arc_attack.after(update_player_facing).in_set(CombatSet::Damage),
                player_death.in_set(CombatSet::Death),
                // XP lands the same frame as a kill: enemy_death emits GainXpEvent in
                // CombatSet::Death, so consume it after that set runs.
                gain_experience.after(CombatSet::Death),
                apply_level_up_reward.after(gain_experience),
            ).run_if(in_state(GameState::InRun)),
        );
        app.add_systems(PostUpdate, draw_player_facing);
    }
}