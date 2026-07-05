// Encounter lifecycle scenarios (Phase 7) — the run flow end-to-end through the headless sim: a
// seeded roster spawns, objectives complete, the branch picker advances the node, teardown is clean,
// depth scaling is driven by the encounter, and the ThroneRoom curse + reward fire. The golden
// campaign never starts a run, so none of this touches the golden master.

use rust_game::game::state::GameState;
use rust_game::sim::Sim;
use rust_game::world::graph::{EncounterType, ObjectiveType};

fn started(seed: u64) -> Sim {
    let mut sim = Sim::new_arena(seed);
    sim.start_run(seed);
    sim
}

/// Loads a synthetic encounter over a started run (keeps RunState; overrides CurrentEncounter).
fn with_encounter(seed: u64, encounter: EncounterType, theme: &str, depth: u32, modifier: Option<&str>) -> Sim {
    let mut sim = started(seed);
    sim.set_current_encounter(encounter, Some(theme), depth, modifier);
    sim.step(3);
    sim
}

#[test]
fn encounter_spawns_themed_roster() {
    let roster = |seed: u64| {
        let mut sim = started(seed);
        sim.step(3);
        sim.enemy_count()
    };
    let n = roster(0xA11CE);
    assert!(n > 0, "the entry encounter spawns a themed roster");
    assert_eq!(n, roster(0xA11CE), "same seed ⇒ same roster (seed-deterministic)");
}

#[test]
fn tutorial_map_is_act1_entry() {
    let mut sim = started(0x7);
    sim.step(3);
    assert_eq!(sim.current_act(), Some(1));
    assert_eq!(sim.current_depth(), Some(0), "Act-1 entry is depth 0 ⇒ base stats");
    let dbg = sim.current_encounter_debug().unwrap();
    assert!(dbg.contains("Map"), "entry is a Map: {dbg}");
    assert!(dbg.contains("KillAll"), "tutorial objective is KillAll: {dbg}");
}

#[test]
fn objective_completion_advances_the_node() {
    let mut sim = started(0xB0B);
    sim.step(3);
    let entry = sim.current_node().unwrap();
    assert!(sim.enemy_count() > 0, "roster present");

    sim.kill_all_enemies();
    sim.step(3);
    assert_eq!(sim.game_state(), GameState::MapSelect, "cleared KillAll ⇒ branch picker");

    let reachable = sim.reachable_nodes();
    assert!(!reachable.is_empty(), "entry has reachable branches");

    sim.pick_branch(0);
    sim.step(3);
    assert_eq!(sim.current_node(), Some(reachable[0]), "advanced to the chosen node");
    assert_ne!(sim.current_node(), Some(entry));
    // Loaded into the next encounter (InRun) — or, if the branch was a Merchant, already auto-cleared
    // back to another MapSelect; either way the run advanced without getting stuck.
    assert!(matches!(sim.game_state(), GameState::InRun | GameState::MapSelect));
}

#[test]
fn picking_a_branch_tears_down_the_previous_encounter() {
    // A Survive room completes with its enemies still alive; they persist through MapSelect and are
    // despawned exactly when the next branch is picked (the player entity persists).
    let mut sim = with_encounter(
        0x7EA,
        EncounterType::Map { objective: ObjectiveType::Survive { duration_secs: 0.1 } },
        "sand_dune",
        0,
        None,
    );
    let old = sim.enemy_entities();
    assert!(!old.is_empty(), "survive room spawns enemies");

    sim.step_seconds(0.3);
    assert_eq!(sim.game_state(), GameState::MapSelect, "survive timer completes");
    assert!(!sim.enemy_entities().is_empty(), "survive enemies persist until teardown");

    let player = sim.player();
    sim.pick_branch(0);
    sim.step(3);
    for e in &old {
        assert!(sim.entity_pos(*e).is_none(), "old encounter enemy {e:?} was torn down");
    }
    assert!(sim.entity_pos(player).is_some(), "the player entity persists across encounters");
}

#[test]
fn survive_objective_completes_on_timer() {
    let mut sim = with_encounter(
        0x5A7E,
        EncounterType::Map { objective: ObjectiveType::Survive { duration_secs: 1.0 } },
        "forest",
        0,
        None,
    );
    assert_eq!(sim.game_state(), GameState::InRun, "still fighting before the timer");
    sim.step_seconds(1.2);
    assert_eq!(sim.game_state(), GameState::MapSelect, "survived ⇒ branch picker");
}

#[test]
fn kill_map_boss_completes_on_boss_death() {
    let mut sim = with_encounter(
        0x805,
        EncounterType::Map { objective: ObjectiveType::KillMapBoss { boss_id: "warlord".into() } },
        "sand_dune",
        0,
        None,
    );
    assert_eq!(sim.map_boss_count(), 1, "one tagged MapBoss spawned");
    assert!(sim.enemy_count() > 1, "pack + boss");

    // Kill ONLY the boss — the pack survives, proving KillMapBoss tracks the boss, not a full clear.
    let boss = sim.map_boss_entities()[0];
    sim.deal_damage(boss, 1.0e6);
    sim.step(3);
    assert_eq!(sim.map_boss_count(), 0, "boss dead");
    assert!(sim.enemy_count() > 0, "pack adds are ignored by the objective");
    assert_eq!(sim.game_state(), GameState::MapSelect, "boss death completes the objective");
}

#[test]
fn enemy_scaling_deepens_with_node_depth() {
    // Drive the Phase-5 scaling curve through the real encounter path: a warlord at a deep node has
    // scaled health and a DamageDealtModifier (neither present at depth 0).
    let mut sim = with_encounter(
        0xDEE9,
        EncounterType::Map { objective: ObjectiveType::KillMapBoss { boss_id: "warlord".into() } },
        "sand_dune",
        6,
        None,
    );
    let boss = sim.map_boss_entities()[0];
    let hp = sim.enemy_health(boss).unwrap();
    assert!(hp > 120.0, "depth deepens the warlord's health (base 120, got {hp})");
    let dmg_mult = sim.damage_dealt_modifier(boss);
    assert!(dmg_mult.is_some_and(|m| m > 1.0), "a deeper boss deals scaled damage: {dmg_mult:?}");
}

#[test]
fn defeating_the_act_boss_advances_the_act() {
    // Force an ActBoss (theme None ⇒ the warlord is the act boss), defeat it, and confirm the act
    // increments and a fresh graph loads — no MapSelect (the next act auto-loads its entry).
    let mut sim = started(0xAC7B055);
    sim.set_current_encounter(EncounterType::ActBoss, None, 14, None);
    sim.step(3);
    assert_eq!(sim.current_act(), Some(1));
    assert!(sim.enemy_count() >= 1, "act boss spawned");

    sim.kill_all_enemies();
    sim.step(3);
    assert_eq!(sim.current_act(), Some(2), "act boss defeated ⇒ next act");
    assert_ne!(sim.game_state(), GameState::MapSelect, "act rolls over into its entry, not a picker");
    assert!(sim.has_run(), "the run continues into act 2");
}

#[test]
fn throne_room_applies_curse_and_offers_reward() {
    let sim = with_encounter(
        0x7405,
        EncounterType::ThroneRoom,
        "sand_dune",
        0,
        Some("enemies_deal_double_damage"),
    );
    assert!(sim.room_modifier_count() >= 1, "the ThroneRoom curse is active");
    assert_eq!(sim.game_state(), GameState::TalentPicker, "the kiss opens a Rare-floor reward picker");
}

#[test]
fn throne_curse_doubles_enemy_damage() {
    // Control: a grunt on top of the player lands its base contact damage.
    let base_loss = {
        let mut sim = Sim::new_arena(0xC0DE);
        sim.set_player_health(100.0);
        sim.spawn_grunt((0, 0));
        sim.step(2);
        100.0 - sim.player_health()
    };
    // Curse: "enemies deal double damage" is applied to the Hostile cast → the same hit doubles.
    let cursed_loss = {
        let mut sim = Sim::new_arena(0xC0DE);
        sim.set_player_health(100.0);
        sim.apply_room_curse("enemies_deal_double_damage");
        sim.spawn_grunt((0, 0));
        sim.step(2);
        100.0 - sim.player_health()
    };
    assert!(base_loss > 0.0, "grunt contact lands (base {base_loss})");
    assert!(
        (cursed_loss - 2.0 * base_loss).abs() < 0.01,
        "curse doubles enemy damage (base {base_loss}, cursed {cursed_loss})"
    );
}
