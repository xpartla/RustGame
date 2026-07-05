// Golden scenario — overlay freeze semantics for in-flight combat events (Phase 3.1).
//
// Combat-resolution events (DamageEvent, HealEvent, ApplyStatusEvent, RemoveStatusEvent) are
// registered via AddGameplayEventExt: their buffers only advance during InRun frames, so an
// event written on the frame an overlay opens survives the freeze and resolves on resume.
// Before this, Bevy's unconditional two-frame expiry silently dropped it — a DoT tick emitted
// the frame a level-up opened the TalentPicker simply vanished.
//
// The scenario aligns a bleed tick with a picker-opening level-up: the tick's DamageEvent is
// written in StatusSet::Tick the same frame handle_level_up (after CombatSet::Death) decides
// to open the picker. apply_damage would consume it the NEXT frame — which is now a picker
// frame. The 3 damage must land on the first frame after resume, not disappear.

use bevy::prelude::KeyCode;
use rust_game::game::state::GameState;
use rust_game::sim::Sim;

#[test]
fn dot_tick_pending_when_picker_opens_lands_after_resume() {
    let mut sim = Sim::new_arena(7);
    let player = sim.player();
    // Far away so the level-up's freshly unlocked auto-cast Blood Boil (radius 90) cannot
    // reach it around the resume — the only damage source in this scenario is the bleed.
    let enemy = sim.spawn_grunt((20, 0));
    sim.set_health(enemy, 100.0);

    sim.apply_status(enemy, player, "bleed", 1);
    sim.step(1);
    assert_eq!(sim.enemy_health(enemy), Some(100.0), "no tick yet");

    // Align: the next frame both fires the bleed tick (DamageEvent written) and processes an
    // XP surge to level 7 (picker opens for the owed talent choice).
    sim.hasten_status_tick(enemy, "bleed");
    sim.grant_xp(140);
    sim.step(1);
    assert_eq!(
        sim.enemy_health(enemy),
        Some(100.0),
        "tick was written this frame; apply_damage runs a frame later"
    );

    sim.step(1);
    assert_eq!(sim.game_state(), GameState::TalentPicker, "level-up opened the picker");
    assert_eq!(
        sim.enemy_health(enemy),
        Some(100.0),
        "world frozen: the pending tick is held, not applied"
    );

    // Hold the freeze a while — the pending event must neither apply nor expire.
    sim.step(10);
    assert_eq!(sim.enemy_health(enemy), Some(100.0));

    // Pick the first talent (alternate press/release so repeated frames read as fresh taps).
    let mut held = false;
    for _ in 0..30 {
        if sim.game_state() != GameState::TalentPicker {
            break;
        }
        if held {
            sim.release_key(KeyCode::Digit1);
        } else {
            sim.press_key(KeyCode::Digit1);
        }
        held = !held;
        sim.step(1);
    }
    assert_eq!(sim.game_state(), GameState::InRun, "talent picked, run resumed");

    sim.step(1);
    assert_eq!(
        sim.enemy_health(enemy),
        Some(97.0),
        "the in-flight bleed tick landed after resume instead of expiring"
    );
}
