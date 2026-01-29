use crate::core::models::Song;
use anyhow::Result;
use walkdir::WalkDir;
use std::path::Path;
use crate::utils::SUPPORTED_EXTENSIONS;

pub fn scan_directory(root: &Path) -> Result<Vec<Song>> {
    let songs = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && is_audio_file(e.path()))
        .map(|entry| Song::from_path(entry.path()))
        .collect();

    Ok(songs)
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}
