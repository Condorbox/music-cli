use crate::core::traits::StorageBackend;
use crate::application::state::AppState;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct JsonStorageBackend {
    file_path: PathBuf,
}

impl JsonStorageBackend {
    pub fn new() -> Result<Self> {
        let mut path = dirs::config_dir().context("Could not find config directory")?;
        path.push("music-cli");

        fs::create_dir_all(&path)?;

        path.push("db.json");
        Ok(Self { file_path: path })
    }
}

impl StorageBackend for JsonStorageBackend {
    fn load(&self) -> Result<AppState> {
        if !self.file_path.exists() {
            return Ok(AppState::default());
        }
        let content = fs::read_to_string(&self.file_path)?;
        let state: AppState = serde_json::from_str(&content)?;
        Ok(state)
    }

    fn save(&self, state: &AppState) -> Result<()> {
        let content = serde_json::to_string_pretty(state)?;
        fs::write(&self.file_path, content)?;
        Ok(())
    }
}
