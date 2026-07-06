// Character select (Phase 7.5C) — one card per hero (`GameState::CharacterSelect`).
//
// Display only; input is logic-side (run/systems/menu.rs::handle_character_select_input, which emits
// a StartRunRequest for the picked hero, refusing a locked pick). Cards are rendered in
// `HeroDef::MANIFEST` order so the on-screen numbers match the selection digits. A locked hero (Phase
// 8, §4 — none are locked yet, D3) is greyed via `hero_is_unlocked`. Never runs headless.

use bevy::prelude::*;

use crate::ability::assets::{AbilityDef, AbilityLibrary};
use crate::core::def_library::DefAsset;
use crate::hero::assets::{HeroDef, HeroLibrary};
use crate::meta::state::{hero_is_unlocked, MetaState};
use crate::ui::theme::{self, text};

#[derive(Component)]
pub struct CharacterSelectRoot;

pub fn spawn_character_select(
    mut commands: Commands,
    hero_lib: Res<HeroLibrary>,
    hero_defs: Res<Assets<HeroDef>>,
    ability_lib: Res<AbilityLibrary>,
    ability_defs: Res<Assets<AbilityDef>>,
    meta: Res<MetaState>,
) {
    commands
        .spawn((CharacterSelectRoot, theme::overlay_root(), BackgroundColor(theme::OVERLAY_BG)))
        .with_children(|root| {
            root.spawn(text("CHOOSE YOUR HERO", theme::FS_TITLE, theme::TITLE));

            for (i, (id, _)) in HeroDef::MANIFEST.iter().enumerate() {
                let unlocked = hero_is_unlocked(&meta, id);
                let name_color = if unlocked { theme::TEXT } else { theme::DIM };
                let def = hero_lib.get(id).and_then(|h| hero_defs.get(h));
                let (name, stance, resource, abilities) = match def {
                    Some(d) => {
                        let stance = if d.has_stance {
                            format!(
                                "{} / {}",
                                d.stance_a.clone().unwrap_or_default(),
                                d.stance_b.clone().unwrap_or_default()
                            )
                        } else {
                            "no stance".to_string()
                        };
                        let abilities = d
                            .level_1_abilities
                            .iter()
                            .map(|aid| {
                                ability_lib
                                    .get(aid)
                                    .and_then(|h| ability_defs.get(h))
                                    .map(|a| a.display_name.clone())
                                    .unwrap_or_else(|| aid.clone())
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        (d.display_name.clone(), stance, format!("{:?}", d.resource_model), abilities)
                    }
                    None => (id.to_string(), "?".to_string(), "?".to_string(), String::new()),
                };
                let locked_suffix = if unlocked { String::new() } else { "  (locked)".to_string() };
                root.spawn(text(
                    format!("{}.   {}    [{}]{}", i + 1, name, stance, locked_suffix),
                    theme::FS_BODY,
                    name_color,
                ));
                root.spawn(text(format!("       {resource}   —   {abilities}"), theme::FS_SMALL, theme::DIM));
            }

            root.spawn(text("1 / 2 to choose       ·       Esc — Back", theme::FS_HINT, theme::HINT));
        });
}

pub fn despawn_character_select(mut commands: Commands, root: Query<Entity, With<CharacterSelectRoot>>) {
    for e in &root {
        commands.entity(e).despawn();
    }
}
