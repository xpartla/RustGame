use bevy::math::Vec2;
use bevy::prelude::Commands;
use crate::constants::{PLAYER_HEALTH, PLAYER_RADIUS, PLAYER_SPEED};
use crate::core::components::{Faction, GridPosition, Health, Hurtbox, MoveSpeed, Velocity, WorldPosition};
use crate::core::components::Facing;
use crate::hero::components::{ActiveStance, HeroIdentity, DEFAULT_HERO_ID};
use crate::player::components::{Experience, Player};

/// Spawns the player with logic components only. Visuals (Transform, Mesh2d, material) are
/// attached by the presentation layer (player/systems/visuals.rs) so headless simulations
/// never touch render assets.
///
/// `Health`/`MoveSpeed` seed with the shared constants (`PLAYER_HEALTH`/`PLAYER_SPEED`) — the
/// HeroDef asset loads asynchronously, so `player/systems/base_stats.rs::apply_base_stats`
/// corrects both from `HeroDef.base_stats` the frame it resolves (Phase 9.2, mirrors
/// `ability/plugin.rs::grant_level_1_abilities`'s deferral pattern).
pub fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player,
        Health::new(PLAYER_HEALTH),
        MoveSpeed(PLAYER_SPEED),
        Experience::new(),
        WorldPosition(Vec2::ZERO),
        GridPosition{x:0, y:0},
        Facing(Vec2::default()),
        Velocity::default(),
        // No projectile can hit the player until enemies shoot (Phase 5), but the hurtbox is
        // part of the actor's logic identity, so it spawns with the rest.
        Hurtbox { radius: PLAYER_RADIUS },
        // The player fights for the Friendly faction; enemy abilities target it (Phase 5).
        Faction::Friendly,
        // Hero identity (Phase 4). The Death Knight is the default class; ActiveStance is
        // "default" (it has no Q swap). These drive input-slot resolution and the level-1 grant.
        HeroIdentity(DEFAULT_HERO_ID.to_string()),
        ActiveStance::default(),
    ));
}
