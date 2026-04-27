use crate::models::PersistentAppData;
use tracing::{info, warn};

fn data_file_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("q-stream")
        .join("app_data.json")
}

/// Load persistent app data from disk. Returns default (empty) data if the
/// file does not exist or cannot be parsed.
pub fn load() -> PersistentAppData {
    let path = data_file_path();
    match std::fs::read_to_string(&path) {
        Ok(json) => match serde_json::from_str(&json) {
            Ok(data) => {
                info!("Loaded persistent app data from {}", path.display());
                data
            }
            Err(e) => {
                warn!("app_data.json is malformed, ignoring: {}", e);
                PersistentAppData::default()
            }
        },
        Err(_) => PersistentAppData::default(),
    }
}

/// Persist app data to disk. Silently ignores write errors.
pub fn save(data: &PersistentAppData) {
    let path = data_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(data) {
        Ok(json) => {
            if std::fs::write(&path, json).is_ok() {
                info!("Persistent app data saved to {}", path.display());
            }
        }
        Err(e) => warn!("Failed to serialize app data: {}", e),
    }
}
