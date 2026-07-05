// EnemyDef and ThemeDef — data templates for enemy types and map themes.
//
// Phase 5 makes EnemyDef a live, RON-loaded DefAsset (`.enemy.ron`) — the single source of truth
// per enemy type. It replaces the old compiled `enemy/archetypes.rs`. One RON file carries stats,
// presentation, AI, its ability list, spawn weight, and a scaling curve:
//   base_stats     — max_health / move_speed / size_radius
//   appearance     — shape + (r,g,b) colour, copied into the presentation-only EnemyAppearance
//   ai_behavior    — mapped to the AiBehavior component (melee_chaser / ranged_caster / stationary)
//   abilities      — AbilityIds (auto-cast abilities the enemy carries; e.g. grunt_contact). These
//                    flow through the SAME ability engine as the player's, faction-aware (Phase 5).
//   spawn_weight   — relative frequency for the ambient weighted spawner
//   scaling        — per-depth growth (health/damage/xp); depth is supplied by the encounter
//                    system in Phase 7, so every live spawn passes depth 0 ⇒ base stats (neutral).
//
// ThemeDef stays scaffold-only (no loader) until Phase 7's room/encounter system needs it.
//
// Interactions:
//   - enemy/systems/spawner.rs reads EnemyDef to build the enemy + its ability instances.
//   - enemy/components.rs::AiBehavior::from_id maps the ai_behavior string.
//   - core/def_library.rs: registered via register_def_library::<EnemyDef>().

use bevy::prelude::*;
use crate::ability::assets::AbilityId;
use crate::core::def_library::{DefAsset, DefLibrary};
use crate::enemy::components::EnemyShape;

pub type EnemyId = String;
pub type ThemeId = String;
pub type AiBehaviorId = String;

/// Loaded from assets/enemies/<id>.enemy.ron.
#[derive(Asset, TypePath, Debug, Clone, serde::Deserialize)]
pub struct EnemyDef {
    pub id: EnemyId,
    pub display_name: String,
    pub rarity: EnemyRarity,
    pub base_stats: EnemyBaseStats,
    /// Presentation source (shape + colour), copied into `EnemyAppearance` at spawn.
    pub appearance: EnemyAppearanceDef,
    /// Relative spawn frequency for the ambient weighted spawner (was `EnemyArchetype.weight`).
    pub spawn_weight: u32,
    /// Key mapped to the `AiBehavior` component — governs movement/targeting.
    pub ai_behavior: AiBehaviorId,
    /// Stand-off distance for `ranged_caster` AI (0 for melee/none).
    #[serde(default)]
    pub preferred_range: f32,
    /// Auto-cast abilities this enemy carries (AbilityIds → `.ability.ron`). Spawned as
    /// `AbilityInstance` children; they fire through the shared ability engine, faction-aware.
    pub abilities: Vec<AbilityId>,
    pub xp_value: u32,
    /// ID of a drop table definition (placeholder string until Phase 7/9).
    pub drop_table: String,
    /// Per-depth scaling curve (Phase 5, data-only — no live driver until Phase 7 supplies depth).
    #[serde(default)]
    pub scaling: EnemyScaling,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub enum EnemyRarity {
    Common,
    Elite,
    MapBoss,
    ActBoss,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EnemyBaseStats {
    pub max_health: f32,
    pub move_speed: f32,
    pub size_radius: f32,
}

/// Presentation source for an enemy type. Colour is an (r,g,b) tuple (like `ThemeDef.ambient_tint`)
/// so the RON needs no Bevy `Color` serde support.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EnemyAppearanceDef {
    pub shape: EnemyShape,
    pub color: (f32, f32, f32),
}

impl EnemyAppearanceDef {
    pub fn color(&self) -> Color {
        Color::srgb(self.color.0, self.color.1, self.color.2)
    }
}

/// Per-depth scaling. Each field is an additive fraction per depth step (0.15 = +15%/depth).
/// Applied by `resolve_enemy_stats`; at depth 0 every field is inert (result == base).
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct EnemyScaling {
    pub health_per_depth: f32,
    pub damage_per_depth: f32,
    pub xp_per_depth: f32,
}

/// Enemy stats resolved for a spawn depth. `damage_mult` is delivered via a `DamageDealtModifier`
/// component so ability numbers stay in the ability RON.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedEnemyStats {
    pub max_health: f32,
    pub xp_value: u32,
    pub damage_mult: f32,
}

/// Pure scaling resolver. `base * (1 + growth * depth)` for health/xp, and a damage multiplier.
/// Move speed and size are not scaled. Depth 0 ⇒ base stats + unit multiplier (baseline-neutral).
pub fn resolve_enemy_stats(def: &EnemyDef, depth: u32) -> ResolvedEnemyStats {
    let d = depth as f32;
    ResolvedEnemyStats {
        max_health: def.base_stats.max_health * (1.0 + def.scaling.health_per_depth * d),
        xp_value: (def.xp_value as f32 * (1.0 + def.scaling.xp_per_depth * d)).round() as u32,
        damage_mult: 1.0 + def.scaling.damage_per_depth * d,
    }
}

/// Loaded from assets/themes/<id>.theme.ron (Phase 7). One file per map theme; its enemy pools are
/// drawn by the seeded encounter spawner (RunRng). Adding a theme = one RON file referencing already
/// -loaded EnemyIds — no code. (D4: the shipped pools point at existing enemies until the Phase 9
/// content pass swaps in the designed per-theme rosters — a pure data edit.)
#[derive(Asset, TypePath, Debug, Clone, serde::Deserialize)]
pub struct ThemeDef {
    pub id: ThemeId,
    pub display_name: String,
    /// Pack enemies (weighted-picked per spawn by RunRng).
    pub common_enemy_pool: Vec<EnemyId>,
    /// Enemies eligible for a BossRoom's single boss.
    pub boss_pool: Vec<EnemyId>,
    /// Enemies eligible for a Map's KillMapBoss objective (tagged `MapBoss`).
    pub map_boss_pool: Vec<EnemyId>,
}

/// Resource mapping ThemeId → Handle<ThemeDef>. A `DefLibrary<ThemeDef>` (see core/def_library.rs);
/// populated at startup from `ThemeDef::MANIFEST` via `register_def_library::<ThemeDef>()`. Read by
/// the encounter spawner (enemy/systems/spawner.rs) to build a themed roster.
pub type ThemeLibrary = DefLibrary<ThemeDef>;

impl DefAsset for ThemeDef {
    // Compound `.theme.ron` extension so the loader never collides with plain `.ron` (mirrors
    // `.enemy.ron` / `.ability.ron`).
    const EXTENSIONS: &'static [&'static str] = &["theme.ron"];
    const MANIFEST: &'static [(&'static str, &'static str)] = &[
        ("sand_dune", "themes/sand_dune.theme.ron"),
        ("forest", "themes/forest.theme.ron"),
        ("castle_ruins", "themes/castle_ruins.theme.ron"),
        ("frozen_wasteland", "themes/frozen_wasteland.theme.ron"),
        ("alpine_lakeside", "themes/alpine_lakeside.theme.ron"),
    ];
}

/// The five act themes, one assigned per act at run start (RunRng). Kept here so graph generation
/// (world/graph.rs) and the run-start flow draw from the same list.
pub const THEME_IDS: &[&str] = &[
    "sand_dune",
    "forest",
    "castle_ruins",
    "frozen_wasteland",
    "alpine_lakeside",
];

/// Resource mapping EnemyId → Handle<EnemyDef>. A `DefLibrary<EnemyDef>` (see core/def_library.rs);
/// populated at startup from `EnemyDef::MANIFEST` via `register_def_library::<EnemyDef>()`.
pub type EnemyLibrary = DefLibrary<EnemyDef>;

impl DefAsset for EnemyDef {
    // Compound `.enemy.ron` extension so the loader never collides with plain `.ron`.
    const EXTENSIONS: &'static [&'static str] = &["enemy.ron"];
    const MANIFEST: &'static [(&'static str, &'static str)] = &[
        ("grunt", "enemies/grunt.enemy.ron"),
        ("runner", "enemies/runner.enemy.ron"),
        ("brute", "enemies/brute.enemy.ron"),
        // Ranged demonstrator (Phase 5C).
        ("spitter", "enemies/spitter.enemy.ron"),
        // Placeholder boss (Phase 7, D4) — never ambient-picked (spawn_weight 0); spawned only by
        // the encounter spawner's boss roles (BossRoom / KillMapBoss / ActBoss). Phase 9 replaces
        // the theme boss pools with the real per-theme bosses (a pure data edit).
        ("warlord", "enemies/warlord.enemy.ron"),
    ];
}

#[cfg(test)]
mod tests {
    //! Parse the real EnemyDef RON files through the same `ron::de` path the AssetLoader uses.
    use super::*;

    fn load(rel_path: &str) -> EnemyDef {
        let full = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel_path);
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("read {full}: {e}"));
        ron::de::from_bytes::<EnemyDef>(&bytes)
            .unwrap_or_else(|e| panic!("parse {rel_path}: {e}"))
    }

    fn load_theme(rel_path: &str) -> ThemeDef {
        let full = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel_path);
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("read {full}: {e}"));
        ron::de::from_bytes::<ThemeDef>(&bytes)
            .unwrap_or_else(|e| panic!("parse {rel_path}: {e}"))
    }

    #[test]
    fn warlord_parses_as_placeholder_boss() {
        let def = load("assets/enemies/warlord.enemy.ron");
        assert_eq!(def.id, "warlord");
        assert_eq!(def.rarity, EnemyRarity::MapBoss);
        assert_eq!(def.spawn_weight, 0, "never ambient-picked — boss roles only");
        assert_eq!(def.abilities, vec!["warlord_smash"]);
        assert!(def.base_stats.max_health >= 100.0, "a boss is beefy");
    }

    #[test]
    fn all_five_themes_parse_and_reference_loaded_enemies() {
        // D4: pools point at existing enemies + the warlord boss. Every referenced id must be in
        // the EnemyDef manifest, so the encounter spawner can always resolve them.
        let manifest_ids: Vec<&str> = EnemyDef::MANIFEST.iter().map(|(id, _)| *id).collect();
        for (id, path) in ThemeDef::MANIFEST {
            let theme = load_theme(&format!("assets/{path}"));
            assert_eq!(theme.id, *id, "theme id matches manifest key");
            assert!(!theme.common_enemy_pool.is_empty(), "{id}: non-empty common pool");
            assert!(!theme.boss_pool.is_empty(), "{id}: non-empty boss pool");
            assert!(!theme.map_boss_pool.is_empty(), "{id}: non-empty map-boss pool");
            for enemy_id in theme
                .common_enemy_pool
                .iter()
                .chain(&theme.boss_pool)
                .chain(&theme.map_boss_pool)
            {
                assert!(
                    manifest_ids.contains(&enemy_id.as_str()),
                    "theme '{id}' references unknown enemy '{enemy_id}' (D4: pools must point at loaded enemies)"
                );
            }
        }
    }

    #[test]
    fn grunt_parses_with_declared_stats() {
        let def = load("assets/enemies/grunt.enemy.ron");
        assert_eq!(def.id, "grunt");
        assert_eq!(def.rarity, EnemyRarity::Common);
        assert_eq!(def.base_stats.max_health, 10.0);
        assert_eq!(def.base_stats.move_speed, 15.0);
        assert_eq!(def.base_stats.size_radius, 12.0);
        assert_eq!(def.ai_behavior, "melee_chaser");
        assert_eq!(def.abilities, vec!["grunt_contact"]);
        assert_eq!(def.xp_value, 3);
        assert_eq!(def.spawn_weight, 6);
    }

    #[test]
    fn runner_and_brute_parse() {
        let runner = load("assets/enemies/runner.enemy.ron");
        assert_eq!(runner.base_stats.max_health, 5.0);
        assert_eq!(runner.base_stats.move_speed, 28.0);
        assert_eq!(runner.abilities, vec!["runner_contact"]);
        let brute = load("assets/enemies/brute.enemy.ron");
        assert_eq!(brute.base_stats.max_health, 30.0);
        assert_eq!(brute.xp_value, 8);
        assert_eq!(brute.abilities, vec!["brute_contact"]);
    }

    #[test]
    fn scaling_is_neutral_at_depth_zero_and_grows_with_depth() {
        let def = load("assets/enemies/grunt.enemy.ron");
        let base = resolve_enemy_stats(&def, 0);
        assert_eq!(base.max_health, def.base_stats.max_health, "depth 0 == base health");
        assert_eq!(base.xp_value, def.xp_value, "depth 0 == base xp");
        assert_eq!(base.damage_mult, 1.0, "depth 0 == unit damage multiplier");

        // With the shipped grunt curve, depth 4 grows each axis by 4× its per-depth fraction.
        let deep = resolve_enemy_stats(&def, 4);
        let d = 4.0;
        assert!((deep.max_health - def.base_stats.max_health * (1.0 + def.scaling.health_per_depth * d)).abs() < 1e-4);
        assert!((deep.damage_mult - (1.0 + def.scaling.damage_per_depth * d)).abs() < 1e-6);
        assert!(deep.max_health > base.max_health, "depth deepens ⇒ more health");
    }
}
