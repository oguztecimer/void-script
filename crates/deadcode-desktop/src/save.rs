use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Persisted editor window geometry (position + size).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// User-configurable settings persisted with the save file.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Settings {
    #[serde(default)]
    pub editor_window: Option<WindowGeometry>,
}

/// Pet state persisted between sessions.
///
/// Stored as JSON in the platform-appropriate app data directory:
/// - macOS: `~/Library/Application Support/good-boi/save.json`
/// - Windows: `%APPDATA%\GoodBoi\good-boi\data\save.json`
/// - Linux: `$XDG_DATA_HOME/good-boi/save.json` or `~/.local/share/good-boi/save.json`
#[derive(Serialize, Deserialize, Debug)]
pub struct SaveData {
    pub hunger: f32,
    pub cleanliness: f32,
    pub happiness: f32,
    /// Unix timestamp (seconds) of when the app was last active.
    /// Used to compute offline decay on next launch.
    pub last_active_unix: u64,
    /// User settings. Defaults to `Settings::default()` when loading old save files
    /// that pre-date this field.
    #[serde(default)]
    pub settings: Settings,
}

/// Returns the path to the save file, or `None` if the platform data directory
/// cannot be determined.
pub fn save_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "GoodBoi", "good-boi")?;
    Some(dirs.data_dir().join("save.json"))
}

/// Persist `data` to disk as JSON. All errors are silently ignored so a save
/// failure never crashes the app — the pet simply loses continuity for that
/// session.
pub fn save(data: &SaveData) {
    let path = match save_path() {
        Some(p) => p,
        None => return,
    };

    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write(&path, json);
    }
}

/// Load persisted state from disk. Returns `None` on first launch or if the
/// save file is missing or corrupt.
pub fn load() -> Option<SaveData> {
    let path = save_path()?;
    let contents = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Returns the current Unix timestamp in seconds.
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Computes how many seconds have elapsed since `last_active_unix`, capped at
/// 7 days (604 800 s). Uses `saturating_sub` to guard against backward clock
/// jumps that would otherwise underflow the u64.
///
/// Apply the returned seconds to stat decay — the cap prevents a pet left
/// closed for weeks from hitting rock-bottom instantly.
pub fn elapsed_since(last_active_unix: u64) -> u64 {
    const MAX_OFFLINE_SECS: u64 = 604_800; // 7 days
    now_unix()
        .saturating_sub(last_active_unix)
        .min(MAX_OFFLINE_SECS)
}
