use crate::audio::EqBandParam;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

fn config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("q-stream")
        .join("config.toml")
}

/// User preferences persisted across sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// Audio output volume, 0.0–1.0.
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Selected output device name ("Default" or a device name from get_audio_devices).
    #[serde(default)]
    pub audio_device: Option<String>,
    /// Whether the equalizer is enabled.
    #[serde(default)]
    pub eq_enabled: bool,
    /// EQ bands (5 = standard, 10 = advanced mode).
    #[serde(default)]
    pub eq_bands: Vec<EqBandParam>,
    /// Whether 10-band advanced EQ mode is active.
    #[serde(default)]
    pub eq_advanced: bool,
}

fn default_volume() -> f32 {
    0.7
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            volume: 0.7,
            audio_device: None,
            eq_enabled: false,
            eq_bands: Vec::new(),
            eq_advanced: false,
        }
    }
}

/// Load config from disk. Returns defaults if the file doesn't exist or is malformed.
pub fn load() -> UserConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(toml_str) => match toml::from_str::<UserConfig>(&toml_str) {
            Ok(cfg) => {
                info!("Loaded user config from {}", path.display());
                cfg
            }
            Err(e) => {
                warn!("config.toml malformed, using defaults: {}", e);
                UserConfig::default()
            }
        },
        Err(_) => UserConfig::default(),
    }
}

/// Save config to disk. Silently ignores write errors.
pub fn save(cfg: &UserConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match toml::to_string_pretty(cfg) {
        Ok(toml_str) => {
            if std::fs::write(&path, toml_str).is_ok() {
                info!("User config saved to {}", path.display());
            } else {
                warn!("Failed to write config to {}", path.display());
            }
        }
        Err(e) => warn!("Failed to serialize config: {}", e),
    }
}
