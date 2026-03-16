use crate::core::models::Song;
use anyhow::Result;
use walkdir::WalkDir;
use std::path::Path;
use crate::utils::{SCAN_PROGRESS_INTERVAL, SUPPORTED_EXTENSIONS};

/// Scan `root` recursively for audio files and return them as a `Vec<Song>`
///
/// `on_progress` is called every [`SCAN_PROGRESS_INTERVAL`] songs with the
/// running count, so callers can surface progress to the user without flooding
/// the event channel on large libraries.  Pass `|_| {}` to ignore progress
pub fn scan_directory(root: &Path, on_progress: impl Fn(usize)) -> Result<Vec<Song>> {
    let songs = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && is_audio_file(e.path()))
        .enumerate()
        .map(|(i, entry)| {
            let mut song = Song::from_path(entry.path());
            song.order = i;

            let count = i + 1;
            if count % SCAN_PROGRESS_INTERVAL == 0 {
                on_progress(count);
            }

            song
        })
        .collect();

    Ok(songs)
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}
