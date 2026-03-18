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

/// App state persisted between sessions.
#[derive(Serialize, Deserialize, Debug)]
pub struct SaveData {
    pub last_active_unix: u64,
    #[serde(default)]
    pub settings: Settings,
}

/// Returns the path to the save file.
pub fn save_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "GoodBoi", "good-boi")?;
    Some(dirs.data_dir().join("save.json"))
}

/// Persist `data` to disk as JSON.
pub fn save(data: &SaveData) {
    let path = match save_path() {
        Some(p) => p,
        None => return,
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write(&path, json);
    }
}

/// Load persisted state from disk.
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
