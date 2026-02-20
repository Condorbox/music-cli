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
    
    fn backup_path(&self) -> PathBuf {
        self.file_path.with_extension("json.bak")
    }

    fn backup_corrupted_file(&self) -> Result<()> {
        let backup = self.backup_path();
        fs::copy(&self.file_path, &backup)
            .context("Failed to backup corrupted config file")?;
        Ok(())
    }
}

impl StorageBackend for JsonStorageBackend {
    fn load(&self) -> Result<AppState> {
        if !self.file_path.exists() {
            return Ok(AppState::default());
        }

        let content = fs::read_to_string(&self.file_path)
            .context("Failed to read config file")?;

        // First attempt: full deserialization
        match serde_json::from_str::<AppState>(&content) {
            Ok(state) => Ok(state),
            Err(full_err) => {
                // Second attempt: partial recovery using serde_json::Value
                // This preserves any valid fields (like root_path, songs list)
                // and fills in missing/new fields with defaults.
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(raw_value) => {
                        // The JSON is structurally valid but schema has evolved.
                        // Merge raw value into a default state so new fields get defaults
                        // and existing valid fields are preserved.
                        let default_json = serde_json::to_value(AppState::default())
                            .context("Failed to serialize default state")?;

                        let merged = merge_json(default_json, raw_value);

                        match serde_json::from_value::<AppState>(merged) {
                            Ok(recovered_state) => {
                                eprintln!(
                                    "Warning: Config schema has changed ({}). \
                                     Some settings were reset to defaults.",
                                    full_err
                                );
                                return Ok(recovered_state);
                            }
                            Err(_) => {
                                // Fall through to corruption handling below
                            }
                        }
                    }
                    Err(_) => {
                        // Not even valid JSON —> fall through to corruption handling
                    }
                }

                // Final fallback: file is unrecoverable —> back it up and start fresh
                match self.backup_corrupted_file() {
                    Ok(_) => {
                        eprintln!(
                            "Warning: Config file was corrupted and could not be recovered. \
                             A backup has been saved to '{}'. \
                             Starting with fresh defaults.",
                            self.backup_path().display()
                        );
                    }
                    Err(backup_err) => {
                        eprintln!(
                            "Warning: Config file was corrupted and the backup also failed ({}). \
                             Starting with fresh defaults.",
                            backup_err
                        );
                    }
                }

                Ok(AppState::default())
            }
        }
    }

    fn save(&self, state: &AppState) -> Result<()> {
        let content = serde_json::to_string_pretty(state)
            .context("Failed to serialize application state")?;
        fs::write(&self.file_path, content)
            .context("Failed to write config file")?;
        Ok(())
    }
}

fn merge_json(base: serde_json::Value, patch: serde_json::Value) -> serde_json::Value {
    match (base, patch) {
        (serde_json::Value::Object(mut base_map), serde_json::Value::Object(patch_map)) => {
            for (key, patch_val) in patch_map {
                let merged_val = match base_map.remove(&key) {
                    Some(base_val) => merge_json(base_val, patch_val),
                    None => patch_val, // Key only in patch, keep it (unknown field, serde will ignore)
                };
                base_map.insert(key, merged_val);
            }
            serde_json::Value::Object(base_map)
        }
        // For non-object values, patch wins
        (_base, patch) => patch,
    }
}
