// Local file persistence for MetaState (Phase 8, §2 of docs/phase8-plan.md).
//
// Mirrors the project's logic/presentation split: a pure (de)serialize layer, unit-tested with no
// disk access, and a thin disk layer that the headless sim never touches.
//
//   serialize_meta   / deserialize_meta   — pure, RON, no I/O.
//   save_path                             — env override → platform dir → ./saves. No new crate
//                                            dependency (the plan's `directories`-crate note is an
//                                            easy later swap if this resolver ever proves thin).
//   save_meta_to_disk / load_meta_from_disk — thin wrappers: serialize_/deserialize_ + fs I/O.
//
// Headless safety: these functions are called only from GamePlugin (windowed) — see meta/plugin.rs.
// The sim drives save/resume entirely through the pure layer + the in-memory
// `MetaState.in_progress_run` field; no filesystem, fully deterministic.
//
// Corrupt/missing file → MetaState::default() (first-run behavior, §2). Never panics.

use crate::meta::state::MetaState;
use bevy::prelude::*;
use std::path::PathBuf;

const SAVE_FILE_NAME: &str = "meta.ron";

/// Serializes `MetaState` to RON. Pure — no I/O.
pub fn serialize_meta(meta: &MetaState) -> Result<String, ron::Error> {
    ron::ser::to_string_pretty(meta, ron::ser::PrettyConfig::default())
}

/// Parses a RON string back into `MetaState`. Pure — no I/O; the caller maps `Err` to a default.
pub fn deserialize_meta(s: &str) -> Result<MetaState, ron::de::SpannedError> {
    ron::de::from_str(s)
}

/// Resolves where `meta.ron` lives: `RUSTGAME_SAVE_DIR` env override (tests/CI point this at a
/// temp dir) → a platform-appropriate app-data directory → `./saves` as a final fallback.
pub fn save_path() -> PathBuf {
    if let Ok(dir) = std::env::var("RUSTGAME_SAVE_DIR") {
        return PathBuf::from(dir).join(SAVE_FILE_NAME);
    }
    platform_save_dir().join(SAVE_FILE_NAME)
}

#[cfg(target_os = "windows")]
fn platform_save_dir() -> PathBuf {
    match std::env::var("APPDATA") {
        Ok(appdata) => PathBuf::from(appdata).join("RustGame"),
        Err(_) => PathBuf::from("./saves"),
    }
}

#[cfg(not(target_os = "windows"))]
fn platform_save_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("rustgame");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/rustgame");
    }
    PathBuf::from("./saves")
}

/// Writes `meta` to disk at `save_path()`, creating parent directories as needed. Logs and gives
/// up (never panics) if the write fails — a save failure should not crash the game.
pub fn save_meta_to_disk(meta: &MetaState) {
    let path = save_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            bevy::log::warn!("could not create save directory {parent:?}: {e}");
            return;
        }
    }
    match serialize_meta(meta) {
        Ok(ron) => {
            if let Err(e) = std::fs::write(&path, ron) {
                bevy::log::warn!("failed to write save file {path:?}: {e}");
            }
        }
        Err(e) => bevy::log::warn!("failed to serialize MetaState: {e}"),
    }
}

/// Reads `MetaState` from disk: missing file or corrupt contents both fall back to
/// `MetaState::default()` (first-run behavior) rather than panicking.
pub fn load_meta_from_disk() -> MetaState {
    let path = save_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => deserialize_meta(&contents).unwrap_or_else(|e| {
            bevy::log::warn!("corrupt save file at {path:?} ({e}) — starting fresh");
            MetaState::default()
        }),
        Err(_) => MetaState::default(),
    }
}

/// Windowed-only Startup system (registered by `GamePlugin`, not `GameLogicPlugin`): overrides the
/// in-memory default `MetaState` that `MetaPlugin` already inserted with whatever is actually on
/// disk. Ordering is safe without an explicit `.after(...)`: Startup fully completes (and flushes)
/// before any Update system — the only readers of `MetaState` — ever runs.
pub fn load_meta_startup(mut commands: Commands) {
    commands.insert_resource(load_meta_from_disk());
}

/// Windowed-only autosave (registered by `GamePlugin`): persists `MetaState` to disk whenever it
/// changes (a node-transition snapshot, a completed run's record, a hero unlock). Gated by the
/// caller on `resource_changed::<MetaState>` so the sim — which never registers this system at
/// all — is unaffected either way.
pub fn autosave_meta_to_disk(meta: Res<MetaState>) {
    save_meta_to_disk(&meta);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::def_library::DefAsset;
    use crate::meta::state::RunRecord;
    use std::sync::Mutex;

    // `save_path` reads a process-wide env var, so tests that touch it must not run concurrently.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    #[test]
    fn round_trips_through_serialize_and_deserialize() {
        let mut meta = MetaState::default();
        meta.run_history.push(RunRecord {
            hero_id: "blood_death_knight".to_string(),
            act_reached: 1,
            score: 100,
            timestamp_unix: 1,
        });
        let ron = serialize_meta(&meta).expect("serialize");
        let restored = deserialize_meta(&ron).expect("deserialize");
        assert_eq!(meta.unlocked_heroes, restored.unlocked_heroes);
        assert_eq!(meta.run_history, restored.run_history);
    }

    #[test]
    fn deserialize_rejects_garbage() {
        assert!(deserialize_meta("not valid ron { at all").is_err());
    }

    #[test]
    fn save_path_honors_the_env_override() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("rustgame_test_{}", std::process::id()));
        // SAFETY: guarded by ENV_GUARD; no other thread in this test binary reads/writes this
        // var concurrently, and the process is single-purpose (test binary), so this cannot
        // race with unrelated code.
        unsafe {
            std::env::set_var("RUSTGAME_SAVE_DIR", &dir);
        }
        let path = save_path();
        unsafe {
            std::env::remove_var("RUSTGAME_SAVE_DIR");
        }
        assert_eq!(path, dir.join(SAVE_FILE_NAME));
    }

    #[test]
    fn missing_file_falls_back_to_default() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("rustgame_test_missing_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        unsafe {
            std::env::set_var("RUSTGAME_SAVE_DIR", &dir);
        }
        let meta = load_meta_from_disk();
        unsafe {
            std::env::remove_var("RUSTGAME_SAVE_DIR");
        }
        assert_eq!(meta.unlocked_heroes.len(), crate::hero::assets::HeroDef::MANIFEST.len());
        assert!(meta.run_history.is_empty());
    }

    #[test]
    fn corrupt_file_falls_back_to_default() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("rustgame_test_corrupt_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SAVE_FILE_NAME), "definitely not ron").unwrap();
        unsafe {
            std::env::set_var("RUSTGAME_SAVE_DIR", &dir);
        }
        let meta = load_meta_from_disk();
        unsafe {
            std::env::remove_var("RUSTGAME_SAVE_DIR");
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(meta.run_history.is_empty(), "corrupt file yields a fresh default, not a panic");
    }

    #[test]
    fn save_then_load_round_trips_through_real_disk() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("rustgame_test_roundtrip_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        unsafe {
            std::env::set_var("RUSTGAME_SAVE_DIR", &dir);
        }
        let mut meta = MetaState::default();
        meta.run_history.push(RunRecord {
            hero_id: "mage".to_string(),
            act_reached: 3,
            score: 9999,
            timestamp_unix: 42,
        });
        save_meta_to_disk(&meta);
        let loaded = load_meta_from_disk();
        unsafe {
            std::env::remove_var("RUSTGAME_SAVE_DIR");
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(loaded.run_history, meta.run_history);
    }
}
