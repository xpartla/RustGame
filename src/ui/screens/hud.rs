// In-run HUD (Phase 7.5A) — the persistent on-screen readout during `GameState::InRun`.
//
// Pure presentation (the `ui/` ground rule): every value is read from existing logic state via
// change-detection queries; the HUD owns nothing and mutates no gameplay. It is registered only
// under PresentationPlugin, so the headless sim / golden master never build it (neutral by
// construction — there is nothing to assert headless; visuals are verified on Windows).
//
// Lifecycle: spawned `OnEnter(InRun)`, despawned `OnExit(InRun)`, updated by systems gated on
// `in_state(InRun)`. Booting to the menu (D1) means the run's first `OnEnter(InRun)` (after
// character-select → start_run) is when the HUD first appears — never on the menu.
//
// Elements (§2.3): player health bar, XP bar + level, stance indicator, class-resource slot (shown
// only when a `ClassResource` is present — inert until Phase-9 frost charges), player status row,
// ability slots with cooldown fills, the objective tracker (hidden with no `CurrentEncounter`), and
// a top-center boss bar for any living `MapBoss`.

use bevy::prelude::*;

use crate::ability::assets::{AbilityDef, AbilityLibrary, Activation};
use crate::ability::components::{AbilityCooldown, AbilityInstance};
use crate::core::components::Health;
use crate::enemy::components::MapBoss;
use crate::game::state::GameState;
use crate::hero::assets::{HeroDef, HeroLibrary};
use crate::hero::components::{ActiveStance, ClassResource, HeroIdentity};
use crate::player::components::{Experience, Player};
use crate::run::state::{CurrentEncounter, ObjectiveProgress, RunState};
use crate::status::components::StatusEffectInstance;
use crate::ui::theme::{self, text};
use crate::world::graph::{
    EncounterType, NodeId, RoomModifierDef, RoomModifierLibrary, COLUMNS_PER_ACT,
};

/// Registers the whole HUD: spawn on entering a run, despawn on leaving, and the change-detection
/// update systems while InRun. Kept here (rather than in ui/plugin.rs) so every marker component and
/// system stays private to this module — the HUD leaks no internal types into the crate API.
pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::InRun), spawn_hud);
    app.add_systems(OnExit(GameState::InRun), despawn_hud);
    app.add_systems(
        Update,
        (
            update_health,
            update_xp,
            update_stance,
            update_class_resource,
            update_status_row,
            update_objective,
            rebuild_ability_slots,
            update_ability_cooldowns,
            update_boss_bar,
            update_curse_banner,
        )
            .run_if(in_state(GameState::InRun)),
    );
}

// ── Markers ──────────────────────────────────────────────────────────────────────────────────
#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
struct HealthFill;
#[derive(Component)]
struct HealthText;
#[derive(Component)]
struct XpFill;
#[derive(Component)]
struct LevelText;
#[derive(Component)]
struct StanceText;
#[derive(Component)]
struct ResourceRow;
#[derive(Component)]
struct ResourceFill;
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct ObjectiveText;
/// The container whose children are the per-ability slot cells (rebuilt when the ability set changes).
#[derive(Component)]
struct AbilitySlotsRow;
/// One ability slot cell, tracking which ability entity drives its cooldown fill.
#[derive(Component)]
struct AbilitySlotCell {
    ability: Entity,
}
#[derive(Component)]
struct AbilitySlotFill;

#[derive(Component)]
struct BossBarRoot;
#[derive(Component)]
struct BossFill;
/// The ThroneRoom curse banner (Phase 7.5D) — shown while a curse is active for the current node.
#[derive(Component)]
struct CurseBanner;

const BAR_W: f32 = 240.0;
const BAR_H: f32 = 18.0;

/// A track+fill bar bundle spawned as a child; returns nothing (the caller tags the fill).
fn bar_track() -> (Node, BackgroundColor) {
    (
        Node {
            width: Val::Px(BAR_W),
            height: Val::Px(BAR_H),
            ..default()
        },
        BackgroundColor(theme::TRACK_BG),
    )
}

fn bar_fill(frac: f32, color: Color) -> (Node, BackgroundColor) {
    (
        Node {
            width: Val::Percent(frac.clamp(0.0, 1.0) * 100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(color),
    )
}

// ── Spawn / despawn ────────────────────────────────────────────────────────────────────────────

/// Builds the static HUD skeleton on entering InRun. Values are populated by the update systems on
/// their first run (all use change detection, which fires once for a freshly spawned/added source).
fn spawn_hud(mut commands: Commands) {
    // Bottom-left player panel.
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|root| {
            // Health.
            root.spawn(bar_track()).with_children(|t| {
                t.spawn((HealthFill, bar_fill(1.0, theme::HEALTH_FILL)));
            });
            root.spawn((HealthText, text("HP", theme::FS_SMALL, theme::TEXT)));
            // XP + level.
            root.spawn(bar_track()).with_children(|t| {
                t.spawn((XpFill, bar_fill(0.0, theme::XP_FILL)));
            });
            root.spawn((LevelText, text("Lv 1", theme::FS_SMALL, theme::TEXT)));
            // Stance (blank for non-stance heroes).
            root.spawn((StanceText, text("", theme::FS_SMALL, theme::ACCENT)));
            // Class-resource bar — its row is collapsed unless a ClassResource is present.
            root.spawn((
                ResourceRow,
                Node {
                    width: Val::Px(BAR_W),
                    height: Val::Px(BAR_H),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(theme::TRACK_BG),
            ))
            .with_children(|t| {
                t.spawn((ResourceFill, bar_fill(0.0, theme::RESOURCE_FILL)));
            });
            // Active status effects.
            root.spawn((StatusText, text("", theme::FS_SMALL, theme::DIM)));
        });

    // Bottom-center ability slots.
    commands.spawn((
        HudRoot,
        AbilitySlotsRow,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(16.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-160.0)),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            ..default()
        },
    ));

    // Top-right objective tracker.
    commands.spawn((
        HudRoot,
        ObjectiveText,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            right: Val::Px(16.0),
            ..default()
        },
        Text::new(""),
        TextFont { font_size: theme::FS_HINT, ..default() },
        TextColor(theme::TITLE),
    ));

    // Top-center boss bar (collapsed until a boss lives).
    commands
        .spawn((
            HudRoot,
            BossBarRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-220.0)),
                width: Val::Px(440.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(text("BOSS", theme::FS_HINT, theme::BOSS_FILL));
            root.spawn((
                Node { width: Val::Px(440.0), height: Val::Px(16.0), ..default() },
                BackgroundColor(theme::TRACK_BG),
            ))
            .with_children(|t| {
                t.spawn((BossFill, bar_fill(1.0, theme::BOSS_FILL)));
            });
        });
}

/// Tears down every HUD node on leaving InRun.
fn despawn_hud(mut commands: Commands, roots: Query<Entity, With<HudRoot>>) {
    for e in &roots {
        commands.entity(e).despawn();
    }
}

// ── Update systems (all `in_state(InRun)`) ──────────────────────────────────────────────────────

/// Health bar + "HP cur/max" text, refreshed when the player's `Health` changes.
fn update_health(
    players: Query<&Health, (With<Player>, Changed<Health>)>,
    mut fills: Query<&mut Node, With<HealthFill>>,
    mut texts: Query<&mut Text, With<HealthText>>,
) {
    let Ok(health) = players.single() else { return };
    let frac = if health.max > 0.0 { health.current / health.max } else { 0.0 };
    if let Ok(mut node) = fills.single_mut() {
        node.width = Val::Percent(frac.clamp(0.0, 1.0) * 100.0);
    }
    if let Ok(mut t) = texts.single_mut() {
        *t = Text::new(format!("HP  {:.0} / {:.0}", health.current.max(0.0), health.max));
    }
}

/// XP bar + "Lv N" text, refreshed when the player's `Experience` changes.
fn update_xp(
    players: Query<&Experience, (With<Player>, Changed<Experience>)>,
    mut fills: Query<&mut Node, With<XpFill>>,
    mut texts: Query<&mut Text, With<LevelText>>,
) {
    let Ok(exp) = players.single() else { return };
    let frac = if exp.to_next > 0 { exp.current as f32 / exp.to_next as f32 } else { 0.0 };
    if let Ok(mut node) = fills.single_mut() {
        node.width = Val::Percent(frac.clamp(0.0, 1.0) * 100.0);
    }
    if let Ok(mut t) = texts.single_mut() {
        *t = Text::new(format!("Lv {}   ({}/{})", exp.level, exp.current, exp.to_next));
    }
}

/// Stance indicator — shows the active stance for stance heroes, blank for non-stance heroes
/// (whose stance is always "default").
fn update_stance(
    players: Query<&ActiveStance, (With<Player>, Changed<ActiveStance>)>,
    mut texts: Query<&mut Text, With<StanceText>>,
) {
    let Ok(stance) = players.single() else { return };
    if let Ok(mut t) = texts.single_mut() {
        *t = if stance.0 == "default" {
            Text::new("")
        } else {
            Text::new(format!("Stance: {}", stance.0.to_uppercase()))
        };
    }
}

/// Class-resource bar — revealed only while a `ClassResource` is present on the player (no shipped
/// hero has one yet; Phase-9 frost charges light this up with zero HUD work). Runs every frame so
/// the reveal happens the moment the component is added.
fn update_class_resource(
    players: Query<&ClassResource, With<Player>>,
    mut rows: Query<&mut Node, (With<ResourceRow>, Without<ResourceFill>)>,
    mut fills: Query<&mut Node, With<ResourceFill>>,
) {
    let Ok(mut row) = rows.single_mut() else { return };
    match players.single() {
        Ok(res) if res.max > 0.0 => {
            row.display = Display::Flex;
            if let Ok(mut fill) = fills.single_mut() {
                fill.width = Val::Percent((res.current / res.max).clamp(0.0, 1.0) * 100.0);
            }
        }
        _ => row.display = Display::None,
    }
}

/// Player status row — a compact list of the status effects currently on the player. Rebuilt when
/// any status instance is added or removed (cheap: a handful of instances at most).
fn update_status_row(
    added: Query<(), Added<StatusEffectInstance>>,
    mut removed: RemovedComponents<StatusEffectInstance>,
    players: Query<Entity, With<Player>>,
    instances: Query<&StatusEffectInstance>,
    mut texts: Query<&mut Text, With<StatusText>>,
) {
    // Only rebuild on a change to the status set.
    if added.is_empty() && removed.read().next().is_none() {
        return;
    }
    let Ok(player) = players.single() else { return };
    let mut names: Vec<&str> = instances
        .iter()
        .filter(|i| i.target == player)
        .map(|i| i.def_id.as_str())
        .collect();
    names.sort_unstable();
    if let Ok(mut t) = texts.single_mut() {
        *t = Text::new(if names.is_empty() { String::new() } else { names.join("  ·  ") });
    }
}

/// Objective tracker — "Act A · Node N/COLUMNS · <objective>". Hidden when no run is active (the
/// arena / no-run world). Refreshed every frame (the KillAll count and Survive countdown are live).
fn update_objective(
    current: Option<Res<CurrentEncounter>>,
    run_state: Option<Res<RunState>>,
    enemies: Query<(), With<crate::enemy::components::Enemy>>,
    mut texts: Query<(&mut Text, &mut Node), With<ObjectiveText>>,
) {
    let Ok((mut t, mut node)) = texts.single_mut() else { return };
    let (Some(current), Some(run_state)) = (current, run_state) else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    let objective = match &current.objective {
        ObjectiveProgress::KillAll => format!("Clear the room ({} left)", enemies.iter().count()),
        ObjectiveProgress::KillMapBoss => "Kill the boss".to_string(),
        ObjectiveProgress::Survive { timer } => {
            format!("Survive {:.0}s", timer.remaining_secs().ceil())
        }
        ObjectiveProgress::Rest => "Merchant — rest".to_string(),
    };
    // `column` is 0-based; show a 1-based "node of act length".
    let node_no = current
        .node
        .checked_add(0)
        .map(|_| run_state.act_graph.node(run_state.current_node).map(|n| n.column + 1).unwrap_or(1))
        .unwrap_or(1);
    *t = Text::new(format!(
        "Act {} · Node {}/{}\n{}",
        run_state.current_act, node_no, COLUMNS_PER_ACT, objective
    ));
}

/// Rebuilds the ability-slot cells when the player's owned-ability set changes (an unlock or a
/// removal). Each cell is labeled by its input slot (LMB/RMB/Shift for the active stance, AUTO for
/// auto-cast passives) and tracks its ability entity so `update_ability_cooldowns` can fill it.
#[allow(clippy::too_many_arguments)]
fn rebuild_ability_slots(
    mut commands: Commands,
    mut last: Local<Vec<Entity>>,
    row: Query<Entity, With<AbilitySlotsRow>>,
    cells: Query<Entity, With<AbilitySlotCell>>,
    players: Query<(&HeroIdentity, &ActiveStance), With<Player>>,
    player_e: Query<Entity, With<Player>>,
    instances: Query<(Entity, &AbilityInstance)>,
    hero_lib: Res<HeroLibrary>,
    hero_defs: Res<Assets<HeroDef>>,
    ability_lib: Res<AbilityLibrary>,
    ability_defs: Res<Assets<AbilityDef>>,
) {
    let Ok(player) = player_e.single() else { return };
    let mut owned: Vec<(Entity, String)> = instances
        .iter()
        .filter(|(_, i)| i.owner == player)
        .map(|(e, i)| (e, i.def_id.clone()))
        .collect();
    owned.sort_by(|a, b| a.1.cmp(&b.1));
    let current: Vec<Entity> = owned.iter().map(|(e, _)| *e).collect();
    if current == *last {
        return; // ability set unchanged — leave the cells (cooldowns update separately)
    }
    *last = current;

    let Ok(row) = row.single() else { return };
    for c in &cells {
        commands.entity(c).despawn();
    }

    // Resolve the active stance's slot bindings for LMB/RMB/Shift labeling.
    let (hero_id, stance) = players.single().map(|(h, s)| (h.0.clone(), s.0.clone())).unwrap_or_default();
    let hero_def = hero_lib.get(&hero_id).and_then(|h| hero_defs.get(h));

    commands.entity(row).with_children(|row| {
        for (ability_e, def_id) in &owned {
            let label = slot_label(hero_def, &stance, def_id, &ability_lib, &ability_defs);
            row.spawn((
                AbilitySlotCell { ability: *ability_e },
                Node {
                    width: Val::Px(72.0),
                    height: Val::Px(56.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(theme::SLOT_READY),
            ))
            .with_children(|cell| {
                cell.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(4.0),
                        ..default()
                    },
                    text(label, theme::FS_SMALL, theme::TEXT),
                ));
                // Cooldown veil: a bar that grows from the bottom as the ability cools, shrinking to
                // nothing when ready. `update_ability_cooldowns` drives its height.
                cell.spawn((
                    AbilitySlotFill,
                    Node { width: Val::Percent(100.0), height: Val::Percent(0.0), ..default() },
                    BackgroundColor(theme::SLOT_COOLING),
                ));
            });
        }
    });
    let _ = player;
}

/// Per-frame cooldown fill for each ability slot: a veil that is full right after a cast and drains
/// to empty as the cooldown elapses.
fn update_ability_cooldowns(
    cells: Query<(&AbilitySlotCell, &Children)>,
    cooldowns: Query<&AbilityCooldown>,
    mut fills: Query<&mut Node, With<AbilitySlotFill>>,
) {
    for (cell, children) in &cells {
        let remaining_frac = cooldowns
            .get(cell.ability)
            .map(|cd| {
                if cd.duration <= 0.0 {
                    0.0
                } else {
                    ((cd.duration - cd.elapsed) / cd.duration).clamp(0.0, 1.0)
                }
            })
            .unwrap_or(0.0);
        for child in children.iter() {
            if let Ok(mut node) = fills.get_mut(child) {
                node.height = Val::Percent(remaining_frac * 100.0);
            }
        }
    }
}

/// Top-center boss bar — shown while any `MapBoss` lives, tracking the first one's health.
fn update_boss_bar(
    bosses: Query<&Health, With<MapBoss>>,
    mut roots: Query<&mut Node, (With<BossBarRoot>, Without<BossFill>)>,
    mut fills: Query<&mut Node, With<BossFill>>,
) {
    let Ok(mut root) = roots.single_mut() else { return };
    match bosses.iter().next() {
        Some(health) if health.max > 0.0 => {
            root.display = Display::Flex;
            if let Ok(mut fill) = fills.single_mut() {
                fill.width = Val::Percent((health.current / health.max).clamp(0.0, 1.0) * 100.0);
            }
        }
        _ => root.display = Display::None,
    }
}

/// ThroneRoom curse banner — while the live encounter is a ThroneRoom with a curse, shows the
/// modifier's name + description (from `RoomModifierDef`, the field that exists for exactly this).
/// Spawns/replaces the banner when the shown node changes and clears it otherwise (a `Local` tracks
/// which node it is currently showing, so it is not rebuilt every frame).
fn update_curse_banner(
    mut commands: Commands,
    mut shown_for: Local<Option<NodeId>>,
    current: Option<Res<CurrentEncounter>>,
    banners: Query<Entity, With<CurseBanner>>,
    modifier_lib: Res<RoomModifierLibrary>,
    modifier_defs: Res<Assets<RoomModifierDef>>,
) {
    // The (node, name, description) we should be showing, if the current node is a cursed ThroneRoom.
    let target: Option<(NodeId, String, String)> = current.as_deref().and_then(|c| {
        match (&c.encounter, &c.modifier) {
            (EncounterType::ThroneRoom, Some(id)) => modifier_lib
                .get(id)
                .and_then(|h| modifier_defs.get(h))
                .map(|d| (c.node, d.display_name.clone(), d.description.clone())),
            _ => None,
        }
    });

    let target_node = target.as_ref().map(|(n, _, _)| *n);
    if target_node == *shown_for {
        return; // already showing the right banner (or nothing)
    }
    for e in &banners {
        commands.entity(e).despawn();
    }
    *shown_for = target_node;

    if let Some((_, name, desc)) = target {
        commands
            .spawn((
                HudRoot,
                CurseBanner,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(64.0),
                    left: Val::Percent(50.0),
                    margin: UiRect::left(Val::Px(-240.0)),
                    width: Val::Px(480.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(theme::PANEL_BG),
            ))
            .with_children(|banner| {
                banner.spawn(text(format!("CURSE — {name}"), theme::FS_HEADING, theme::DANGER));
                banner.spawn(text(desc, theme::FS_HINT, theme::TEXT));
            });
    }
}

/// Resolves an ability id to its input-slot label for the active stance (LMB/RMB/Shift), or AUTO for
/// an auto-cast passive, falling back to a short form of the id.
fn slot_label(
    hero_def: Option<&HeroDef>,
    stance: &str,
    def_id: &str,
    ability_lib: &AbilityLibrary,
    ability_defs: &Assets<AbilityDef>,
) -> String {
    if let Some(hero) = hero_def {
        if let Some(m) = hero.stance_slots.iter().find(|m| m.stance == stance) {
            if m.basic.as_deref() == Some(def_id) {
                return "LMB".to_string();
            }
            if m.special.as_deref() == Some(def_id) {
                return "RMB".to_string();
            }
            if m.movement.as_deref() == Some(def_id) {
                return "Shift".to_string();
            }
        }
    }
    let is_auto = ability_lib
        .get(def_id)
        .and_then(|h| ability_defs.get(h))
        .map(|d| d.activation == Activation::AutoCast)
        .unwrap_or(false);
    if is_auto {
        "AUTO".to_string()
    } else {
        def_id.split('_').next().unwrap_or(def_id).to_uppercase()
    }
}
