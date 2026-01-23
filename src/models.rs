use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AppConfig {
    pub root_path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Song {
    pub path: PathBuf,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppState {
    pub config: AppConfig,
    pub library: Vec<Song>,
}