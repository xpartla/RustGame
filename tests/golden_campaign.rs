// Golden-master campaign — the broadest backward-compatibility net.
//
// A deterministic scripted bot plays 30 simulated seconds against scripted enemy waves:
// it chases the nearest enemy, casts Death Strike on cooldown, kites when hurt, and picks
// the first talent option whenever the picker opens. Once per simulated second a snapshot
// of observable game state is recorded. The full trace must match the committed baseline
// exactly.
//
// If this test fails after a code change:
//   1. Read the printed diff — it names the first diverging second and field.
//   2. Decide: regression (fix the code) or intentional behavior change (must be declared
//      in the CHANGELOG). Only then regenerate the baseline:
//         UPDATE_GOLDEN=1 cargo test --test golden_campaign
//   3. Commit the new baseline together with the change that explains it.
//
// The baseline is platform/toolchain-sensitive in principle (f32 accumulation, HashMap seeds
// are avoided but rand/bevy versions matter — Cargo.lock pins them). Regenerate deliberately,
// never blindly.

use bevy::math::Vec2;
use bevy::prelude::KeyCode;
use rust_game::enemy::archetypes::archetypes;
use rust_game::game::state::GameState;
use rust_game::player::components::Experience;
use rust_game::sim::Sim;

const GOLDEN_SEED: u64 = 0xC0FFEE;
const CAMPAIGN_FRAMES: usize = 1800; // 30 simulated seconds at 60 fps
const SNAPSHOT_EVERY: usize = 60;
const BASELINE_PATH: &str = "tests/golden/campaign_baseline.ron";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Snapshot {
    frame: usize,
    state: String,
    /// Player health, rounded to 2 decimals (f32 noise guard).
    hp: f32,
    level: u32,
    xp: u32,
    enemies: usize,
    abilities: usize,
    talents: u32,
    /// Player position rounded to 1 decimal.
    px: f32,
    py: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct CampaignTrace {
    seed: u64,
    map_signature: u64,
    snapshots: Vec<Snapshot>,
}

fn round(v: f32, places: i32) -> f32 {
    let m = 10f32.powi(places);
    (v * m).round() / m
}

fn snapshot(sim: &mut Sim, frame: usize) -> Snapshot {
    let state = format!("{:?}", sim.game_state());
    let (hp, px, py) = if sim.try_player().is_some() {
        let hp = sim.player_health();
        let pos = sim.player_pos();
        (round(hp, 2), round(pos.x, 1), round(pos.y, 1))
    } else {
        (0.0, 0.0, 0.0)
    };
    let (level, xp) = match sim.try_player() {
        Some(p) => {
            let e = sim.world().get::<Experience>(p).unwrap();
            (e.level, e.current)
        }
        None => (0, 0),
    };
    let talents: u32 = sim
        .acquired_talents()
        .iter()
        .map(|(_, count)| *count as u32)
        .sum();
    Snapshot {
        frame,
        state,
        hp,
        level,
        xp,
        enemies: sim.enemy_count(),
        abilities: sim.owned_abilities().len(),
        talents,
        px,
        py,
    }
}

/// One frame of the scripted bot: seek nearest enemy, attack in range, kite when hurt,
/// pick talents when offered. `digit_held` alternates press/release so repeated picker
/// choices register as distinct key taps.
fn bot_frame(sim: &mut Sim, digit_held: &mut bool) {
    if sim.game_state() == GameState::TalentPicker {
        if *digit_held {
            sim.release_key(KeyCode::Digit1);
        } else {
            sim.press_key(KeyCode::Digit1);
        }
        *digit_held = !*digit_held;
        return;
    }
    if *digit_held {
        sim.release_key(KeyCode::Digit1);
        *digit_held = false;
    }

    let Some(player) = sim.try_player() else {
        return;
    };
    let ppos = sim.player_pos();

    // Nearest enemy by scanning positions (deterministic: query iteration order is stable).
    let world = sim.world_mut();
    let mut nearest: Option<(Vec2, f32)> = None;
    let mut query = world.query_filtered::<&rust_game::core::components::WorldPosition, bevy::prelude::With<rust_game::enemy::components::Enemy>>();
    for pos in query.iter(world) {
        let d = pos.0.distance(ppos);
        if nearest.map_or(true, |(_, nd)| d < nd) {
            nearest = Some((pos.0, d));
        }
    }
    let _ = player;

    for key in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD] {
        sim.release_key(key);
    }

    let Some((epos, dist)) = nearest else {
        return; // no enemies: idle
    };

    let hurt = sim.player_health() < 30.0;
    let dir = if hurt { ppos - epos } else { epos - ppos };
    let engaged = !hurt && dist < 25.0;
    if !engaged {
        if dir.x > 8.0 {
            sim.press_key(KeyCode::KeyD);
        } else if dir.x < -8.0 {
            sim.press_key(KeyCode::KeyA);
        }
        if dir.y > 8.0 {
            sim.press_key(KeyCode::KeyW);
        } else if dir.y < -8.0 {
            sim.press_key(KeyCode::KeyS);
        }
    }

    if dist < 50.0 {
        sim.set_player_facing(epos - ppos);
        sim.trigger_ability("death_strike");
    }
}

/// Scripted one-off events. At 10s a large XP surge (as if a boss died) pushes the player
/// across the whole ability-unlock band into the talent-choice phase, so the trace also
/// covers multi-level frames, band unlocks, the TalentPicker round-trip, and talent picks.
fn scripted_events(sim: &mut Sim, frame: usize) {
    if frame == 600 {
        if let Some(player) = sim.try_player() {
            sim.world_mut()
                .send_event(rust_game::core::events::GainXpEvent { target: player, amount: 140 });
        }
    }
}

/// Scripted waves: 3 grunts every 5s, plus a brute every 15s, on fixed tile offsets from
/// the player (clamped to the arena).
fn spawn_waves(sim: &mut Sim, frame: usize) {
    let clamp = |v: i32| v.clamp(-38, 38);
    let ptile = {
        let p = sim.player_pos();
        ((p.x / 32.0).round() as i32, (p.y / 32.0).round() as i32)
    };
    if frame % 300 == 0 {
        for (dx, dy) in [(6, 0), (0, 6), (-6, 0)] {
            sim.spawn_grunt((clamp(ptile.0 + dx), clamp(ptile.1 + dy)));
        }
    }
    if frame % 900 == 0 {
        sim.spawn_enemy(&archetypes()[2], (clamp(ptile.0), clamp(ptile.1 - 6)));
    }
}

fn run_campaign() -> CampaignTrace {
    let mut sim = Sim::new_arena(GOLDEN_SEED);
    let map_signature = sim.tilemap_signature();
    let mut snapshots = Vec::new();
    let mut digit_held = false;

    for frame in 0..CAMPAIGN_FRAMES {
        spawn_waves(&mut sim, frame);
        scripted_events(&mut sim, frame);
        bot_frame(&mut sim, &mut digit_held);
        sim.step(1);
        if (frame + 1) % SNAPSHOT_EVERY == 0 {
            snapshots.push(snapshot(&mut sim, frame + 1));
        }
    }

    CampaignTrace { seed: GOLDEN_SEED, map_signature, snapshots }
}

#[test]
fn campaign_matches_golden_baseline() {
    let trace = run_campaign();
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), BASELINE_PATH);

    if std::env::var("UPDATE_GOLDEN").is_ok() {
        let ron = ron::ser::to_string_pretty(&trace, ron::ser::PrettyConfig::default())
            .expect("serialize trace");
        std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).unwrap();
        std::fs::write(&path, ron).expect("write baseline");
        eprintln!("golden baseline updated: {path}");
        return;
    }

    let baseline_ron = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("no golden baseline at {path} ({e}) — generate one with UPDATE_GOLDEN=1")
    });
    let baseline: CampaignTrace =
        ron::de::from_str(&baseline_ron).expect("parse committed baseline");

    assert_eq!(
        trace.map_signature, baseline.map_signature,
        "map generation diverged from baseline for the same seed"
    );
    for (current, expected) in trace.snapshots.iter().zip(baseline.snapshots.iter()) {
        assert_eq!(
            current, expected,
            "campaign diverged at frame {} (≈ second {})",
            expected.frame,
            expected.frame / 60
        );
    }
    assert_eq!(trace.snapshots.len(), baseline.snapshots.len());
}

/// Determinism guard: two fresh runs of the identical campaign must produce identical traces.
/// If this fails, something nondeterministic crept into the InRun simulation (thread_rng in a
/// gameplay system, unordered RunRng consumers, iteration-order dependence...).
#[test]
fn campaign_is_reproducible_within_a_build() {
    let a = run_campaign();
    let b = run_campaign();
    assert_eq!(a, b, "identical seeds + scripts must replay identically");
}
