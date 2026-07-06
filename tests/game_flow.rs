// Golden scenarios — the game-flow state machine (Phase 7.5B/C): death → GameOver, restart, pause,
// and character-select. The screens themselves are presentation-only (never headless); these drive
// their *logic* — the state transitions and the run-reset primitive — through the real input paths.

use bevy::prelude::KeyCode;
use rust_game::game::state::GameState;
use rust_game::sim::Sim;

/// Killing the player captures a defeat summary and freezes into GameOver (was: a bare despawn that
/// left the world running). Declared behavior change — the campaign bot never dies, so neutral.
#[test]
fn player_death_enters_game_over() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    sim.deal_damage(player, 150.0);
    sim.step(2);

    assert!(sim.try_player().is_none(), "player despawned at 0 hp");
    assert_eq!(sim.game_state(), GameState::GameOver, "death enters GameOver");
    assert_eq!(sim.game_over_victory(), Some(false), "a defeat summary was captured");
}

/// Death → restart request boots a clean, fresh, deterministic run: the reset tears down *every*
/// run-scoped entity (including the dead player's orphaned ability-instance entities) before
/// re-spawning, so the world momentarily holds zero ability instances; then a fresh level-1 player
/// and the entry encounter load.
#[test]
fn restart_after_death_boots_a_fresh_run() {
    let mut sim = Sim::new_arena(42);
    sim.start_run(1000);
    sim.step(6); // load the entry encounter (roster + the deferred level-1 grant)
    assert!(sim.has_run());
    assert!(sim.enemy_count() > 0, "the entry encounter spawned a roster");
    // Capture every run-scoped ability-instance entity (the player's + the roster's) so we can
    // assert the reset despawns them (the fresh run spawns brand-new instance entities).
    let old_instances = sim.ability_instance_entities();
    assert!(!old_instances.is_empty(), "player + roster carry ability instances");

    // Die.
    let player = sim.player();
    sim.deal_damage(player, 1.0e6);
    sim.step(2);
    assert_eq!(sim.game_state(), GameState::GameOver);
    assert!(sim.try_player().is_none());

    // Restart through the real StartRunRequest → apply_start_run_request path with a fixed seed.
    sim.request_start_run("blood_death_knight", 2024);
    sim.step(9); // reset frame + settle: fresh grant + load_encounter for the fresh run
    assert!(
        old_instances.iter().all(|e| !sim.entity_exists(*e)),
        "the reset despawned every old ability instance (dead player's orphans + old roster's)"
    );

    assert_eq!(sim.game_state(), GameState::InRun);
    assert!(sim.has_run(), "a fresh run is active");
    assert!(sim.try_player().is_some(), "a fresh player exists");
    assert_eq!(sim.player_level(), 1, "the fresh player starts at level 1");
    assert!(sim.encounter_spawned(), "the entry encounter loaded");

    // Reproducibility: an independently-driven identical restart lands on the same node.
    let mut other = Sim::new_arena(42);
    other.start_run(1000);
    other.step(6);
    let p2 = other.player();
    other.deal_damage(p2, 1.0e6);
    other.step(2);
    other.request_start_run("blood_death_knight", 2024);
    other.step(9);
    assert_eq!(sim.current_node(), other.current_node(), "restart is seed-deterministic");
    assert_eq!(sim.current_depth(), other.current_depth());
    assert!(other.has_run() && other.try_player().is_some(), "control run booted a fresh player");
}

/// Esc pauses; a combat event written the frame the pause opens survives the freeze and resolves on
/// resume — the `add_gameplay_event` contract (freeze.rs), now exercised for `Paused`.
#[test]
fn esc_toggles_pause_and_preserves_combat_events() {
    let mut sim = Sim::new_arena(7);
    let player = sim.player();
    let enemy = sim.spawn_grunt((20, 0)); // far away: nothing else can damage it
    sim.set_health(enemy, 100.0);

    sim.apply_status(enemy, player, "bleed", 1);
    sim.step(1);
    assert_eq!(sim.enemy_health(enemy), Some(100.0), "no tick yet");

    // This frame writes the bleed tick's DamageEvent *and* presses Esc (which queues the pause
    // transition, applied next frame — so the frame that would apply the damage is frozen instead).
    sim.hasten_status_tick(enemy, "bleed");
    sim.press_key(KeyCode::Escape);
    sim.step(1);
    sim.release_key(KeyCode::Escape);
    assert_eq!(sim.enemy_health(enemy), Some(100.0), "tick written this frame; apply runs later");

    // The pause transition applies now; the pending tick is held (apply_damage is InRun-gated).
    sim.step(1);
    assert_eq!(sim.game_state(), GameState::Paused, "Esc opened the pause overlay");
    assert_eq!(sim.enemy_health(enemy), Some(100.0), "pending tick held, not applied");

    // Hold the freeze — the pending event must neither apply nor expire.
    sim.step(30);
    assert_eq!(sim.enemy_health(enemy), Some(100.0), "world frozen while paused");

    // Resume with Esc; the transition applies next frame, then the held tick lands.
    sim.tap_key(KeyCode::Escape);
    sim.step(1);
    assert_eq!(sim.game_state(), GameState::InRun, "Esc resumed the run");
    sim.step(1);
    assert_eq!(sim.enemy_health(enemy), Some(97.0), "the in-flight bleed tick landed after resume");
}

/// While paused, the world does not tick: an enemy spawned in the pause does not move, and the
/// player takes no contact damage, across many frames.
#[test]
fn pause_does_not_tick_the_world() {
    let mut sim = Sim::new_arena(11);
    sim.tap_key(KeyCode::Escape);
    sim.step(1); // let the pause transition apply
    assert_eq!(sim.game_state(), GameState::Paused);

    // Spawn a chaser adjacent to the player *during* the pause; nothing should move it or let it hit.
    let enemy = sim.spawn_grunt((2, 0));
    let enemy_pos_before = sim.entity_pos(enemy);
    let hp_before = sim.player_health();

    sim.step(120); // 2 simulated seconds — a grunt would otherwise chase + hit ~twice
    assert_eq!(sim.entity_pos(enemy), enemy_pos_before, "enemy did not move while paused");
    assert_eq!(sim.player_health(), hp_before, "no contact damage while paused");
    assert_eq!(sim.enemy_count(), 1, "no spawns/despawns while paused");
}

/// Menu → CharacterSelect → pick the Mage → the run boots as the Mage (level-1 grant ran) and the
/// entry encounter is live. Booting to a menu is windowed-only (D1); this drives the same logic
/// states, proving the whole flow is sim-able through the real input systems. (Phase 7.5C.)
#[test]
fn character_select_starts_the_chosen_hero() {
    let mut sim = Sim::new_arena(5);
    sim.enter_menu();
    assert_eq!(sim.game_state(), GameState::Menu);

    sim.tap_key(KeyCode::Enter); // Menu: "New Run"
    sim.step(1); // apply Menu → CharacterSelect
    assert_eq!(sim.game_state(), GameState::CharacterSelect);

    sim.select_hero_index(1); // the Mage is HeroDef::MANIFEST entry 1
    sim.step(10); // StartRunRequest → reset + start_run, then settle the grant + encounter

    assert_eq!(sim.game_state(), GameState::InRun);
    assert_eq!(sim.hero_id(), "mage", "the run started as the chosen hero");
    assert_eq!(sim.active_stance(), "fire", "a stance hero starts in its stance_a");
    assert!(sim.owned_abilities().iter().any(|a| a == "fireblast"), "Mage level-1 grant ran");
    assert!(sim.has_run() && sim.encounter_spawned(), "the entry encounter loaded");
}

/// Boot reaches Login before the main menu (Phase 8, D4) — any key advances Login → Menu. Booting
/// to Login is windowed-only; this drives the same logic states headlessly, like the Menu boot test
/// above did before Login existed.
#[test]
fn boot_reaches_login_then_menu() {
    let mut sim = Sim::new_arena(6);
    sim.enter_login();
    assert_eq!(sim.game_state(), GameState::Login);

    sim.tap_key(KeyCode::Enter);
    sim.step(1); // apply Login → Menu

    assert_eq!(sim.game_state(), GameState::Menu, "any key at Login advances to the main menu");
}

/// Resume Run from the main menu (Phase 8, §3.2): clear an encounter (saving), return to the menu,
/// and Resume rehydrates the exact saved run rather than starting a fresh one.
#[test]
fn resume_from_the_main_menu_enters_in_run_with_the_saved_run() {
    let mut sim = Sim::new_arena(9);
    sim.start_run(4444);
    sim.step(3);
    sim.kill_all_enemies();
    sim.step(3); // -> save into MetaState.in_progress_run + MapSelect
    let saved_node = sim.current_node();
    let saved_act = sim.current_act();
    assert!(sim.meta().in_progress_run.is_some());

    // Simulate returning to the main menu with the run still saved (e.g. after quitting and
    // relaunching — MetaState persists independently of any live RunState/CurrentEncounter).
    sim.enter_menu();
    assert_eq!(sim.game_state(), GameState::Menu);

    sim.tap_key(KeyCode::Digit2); // "2. Resume Run"
    sim.step(9); // apply_resume_request -> resume_run, then settle the re-grant + load_encounter

    assert_eq!(sim.game_state(), GameState::InRun, "Resume Run entered the run");
    assert!(sim.has_run());
    assert_eq!(sim.current_node(), saved_node, "resumed at the saved node");
    assert_eq!(sim.current_act(), saved_act);
    assert!(sim.encounter_spawned(), "the saved node's room loaded");
}
