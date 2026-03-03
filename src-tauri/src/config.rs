use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub db_path: String,
    // Legacy fields — read for migration, never written back
    #[serde(default)]
    pub hotkey: Option<String>,
    #[serde(default)]
    pub ai_provider: Option<String>,
    #[serde(default)]
    pub ai_api_key: Option<String>,
    #[serde(default)]
    pub ai_base_url: Option<String>,
    #[serde(default)]
    pub task_archive_delay_secs: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MinimalConfig {
    db_path: String,
}

impl AppConfig {
    fn with_defaults(data_dir: &Path) -> Self {
        let db_path = data_dir.join("aihelper.db");
        Self {
            db_path: db_path.to_string_lossy().to_string(),
            hotkey: None,
            ai_provider: None,
            ai_api_key: None,
            ai_base_url: None,
            task_archive_delay_secs: None,
        }
    }

    pub fn load_or_create(config_dir: &Path, data_dir: &Path) -> Result<(Self, PathBuf), String> {
        fs::create_dir_all(config_dir).map_err(|e| format!("Failed to create config dir: {e}"))?;
        fs::create_dir_all(data_dir).map_err(|e| format!("Failed to create data dir: {e}"))?;

        let config_path = config_dir.join("config.toml");

        let config = if config_path.exists() {
            let contents = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config: {e}"))?;
            toml::from_str(&contents).map_err(|e| format!("Failed to parse config: {e}"))?
        } else {
            let config = Self::with_defaults(data_dir);
            // Write minimal config for fresh installs
            let minimal = MinimalConfig {
                db_path: config.db_path.clone(),
            };
            let contents = toml::to_string_pretty(&minimal)
                .map_err(|e| format!("Failed to serialize config: {e}"))?;
            fs::write(&config_path, contents)
                .map_err(|e| format!("Failed to write config: {e}"))?;
            config
        };

        Ok((config, config_path))
    }

    /// Rewrites config.toml to contain only db_path (strips legacy fields after migration).
    pub fn rewrite_minimal(path: &Path, db_path: &str) -> Result<(), String> {
        let minimal = MinimalConfig {
            db_path: db_path.to_string(),
        };
        let contents = toml::to_string_pretty(&minimal)
            .map_err(|e| format!("Failed to serialize minimal config: {e}"))?;
        fs::write(path, contents).map_err(|e| format!("Failed to write minimal config: {e}"))?;
        Ok(())
    }
}
