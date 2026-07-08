// Applies the active hero's `HeroDef.base_stats` to the player (Phase 9.2, closing the last open
// §8.5 tech-debt row). `spawn_player` seeds `Health`/`MoveSpeed` with the shared
// `PLAYER_HEALTH`/`PLAYER_SPEED` constants because the `HeroDef` asset loads asynchronously;
// `apply_base_stats` corrects both the frame the def resolves, exactly mirroring
// `ability/plugin.rs::grant_level_1_abilities`'s deferral pattern (a `BaseStatsApplied` marker
// makes it fire exactly once per player).
//
// This one system covers every spawn path — initial boot (`spawn_player`), a restart
// (`run/systems/reset.rs::respawn_player`), and a resume (`run/systems/persistence.rs::resume_run`)
// — because all three end with a fresh `Player` entity carrying `HeroIdentity` and no
// `BaseStatsApplied` marker; none of those call sites need to change.

use bevy::prelude::*;
use crate::core::components::{BaseHealth, Health, MoveSpeed};
use crate::hero::assets::{HeroDef, HeroLibrary, ResourceModel};
use crate::hero::components::{Charges, HeroIdentity};
use crate::player::components::Player;

/// Marker: this player's `Health`/`MoveSpeed` have been set from its active `HeroDef.base_stats`.
/// Once present, neither this system nor `run/systems/reset.rs::respawn_player` (which sets the
/// marker itself, synchronously — see its doc comment) will touch `Health.max`/`MoveSpeed` again,
/// so a subsequent `resume_run`'s explicit `health.current = saved.player_health` is never
/// clobbered by this system re-detecting an "unmarked" player on a later frame.
#[derive(Component, Debug)]
pub struct BaseStatsApplied;

/// Resolves `hero_id`'s `(max_health, move_speed)` from its loaded `HeroDef`, or `None` if the
/// hero library/asset hasn't resolved yet. Shared by the deferred Update system below and by
/// `run/systems/reset.rs::respawn_player`'s synchronous application.
pub fn resolve_base_stats(hero_library: &HeroLibrary, hero_defs: &Assets<HeroDef>, hero_id: &str) -> Option<(f32, f32)> {
    let handle = hero_library.get(hero_id)?;
    let def = hero_defs.get(handle)?;
    Some((def.base_stats.max_health, def.base_stats.move_speed))
}

/// Resolves `hero_id`'s charge cap if its `resource_model` is `Charges { max }` (Phase 9.4 — the
/// Druid's Enhanced-attack state). `None` for every other resource model, or if the def hasn't
/// loaded yet — mirrors `resolve_base_stats`'s shape so both deferred-application call sites stay
/// symmetric.
pub fn resolve_charges_max(hero_library: &HeroLibrary, hero_defs: &Assets<HeroDef>, hero_id: &str) -> Option<u32> {
    let handle = hero_library.get(hero_id)?;
    let def = hero_defs.get(handle)?;
    match def.resource_model {
        ResourceModel::Charges { max } => Some(max),
        _ => None,
    }
}

/// Deferred, idempotent per-player application of `HeroDef.base_stats` — covers the one spawn path
/// that cannot apply it synchronously: the very first boot spawn (`OnEnter(InRun)`'s
/// `spawn_player`), where the `HeroDef` asset may not have finished loading yet. Every other spawn
/// path (`respawn_player`, reused by restart + resume) applies it synchronously and sets the marker
/// itself, so this system skips them. Ungated by `GameState` (mirrors `grant_level_1_abilities`):
/// a player can exist a frame before `InRun`-gated systems would otherwise see it.
pub fn apply_base_stats(
    mut commands: Commands,
    hero_library: Res<HeroLibrary>,
    hero_defs: Res<Assets<HeroDef>>,
    mut players: Query<
        (Entity, &HeroIdentity, &mut Health, &mut MoveSpeed),
        (With<Player>, Without<BaseStatsApplied>),
    >,
) {
    for (entity, hero_id, mut health, mut move_speed) in &mut players {
        let Some((max_health, speed)) = resolve_base_stats(&hero_library, &hero_defs, &hero_id.0) else {
            continue;
        };
        *health = Health::new(max_health);
        move_speed.0 = speed;
        commands.entity(entity).insert((BaseStatsApplied, BaseHealth(max_health)));
        if let Some(max_charges) = resolve_charges_max(&hero_library, &hero_defs, &hero_id.0) {
            commands.entity(entity).insert(Charges::new(max_charges));
        }
    }
}
