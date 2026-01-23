use crate::models::AppState;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct StoreManager {
    file_path: PathBuf,
}

impl StoreManager {
    pub fn new() -> Result<Self> {
        let mut path = dirs::config_dir().context("Could not find config directory")?;
        path.push("music-cli");

        fs::create_dir_all(&path)?;

        path.push("db.json");
        Ok(Self { file_path: path })
    }

    pub fn load(&self) -> Result<AppState> {
        if !self.file_path.exists() {
            return Ok(AppState::default());
        }
        let content = fs::read_to_string(&self.file_path)?;
        let state: AppState = serde_json::from_str(&content)?;
        Ok(state)
    }

    pub fn save(&self, state: &AppState) -> Result<()> {
        let content = serde_json::to_string_pretty(state)?;
        fs::write(&self.file_path, content)?;
        Ok(())
    }
}