use std::fs;
use anyhow::Context;
use walkdir::WalkDir;
use anyhow::Result;

use crate::library::store::StoreManager;


pub fn handle_set_path(path_str: String, store: &StoreManager) -> Result<()> {
    let path = std::path::PathBuf::from(&path_str);

    if !path.exists() || !path.is_dir() {
        anyhow::bail!("The path provided is not a valid directory.");
    }

    let mut state = store.load()?;
    state.config.root_path = Some(fs::canonicalize(path)?); // Store absolute path

    store.save(&state)?;
    println!("Music path updated to: {:?}", state.config.root_path);
    // TODO Create library
    Ok(())
}

// TODO Maybe add an optional customPath
pub fn handle_playlist(store: &StoreManager) -> Result<()> {
    let state = store.load()?;
    if state.library.is_empty() {
        anyhow::bail!("Library is empty. Run 'music-cli refresh' or set a path.");
    }
    let songs_to_play: Vec<crate::models::Song> = state.library;

    if songs_to_play.is_empty() {
        println!("No songs found to play.");
        return Ok(());
    }

    // TODO: Play the songs
    println!("Queueing {} songs...", songs_to_play.len());

    for song in songs_to_play {
        println!("{}", song.title);
    }

    Ok(())
}

pub fn handle_refresh(store: &StoreManager) -> Result<()> {
    let mut state = store.load()?;
    let root = state.config.root_path.as_ref()
        .context("No music path set. Run 'music-cli path <DIR>' first.")?;

    println!("Scanning {:?}...", root);

    let new_library = scan_directory(root)?;
    let count = new_library.len();

    state.library = new_library;
    store.save(&state)?;

    println!("Refresh complete. Found {} songs.", count);
    Ok(())
}

fn scan_directory(root: &std::path::Path) -> Result<Vec<crate::models::Song>> {
    let mut songs = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if is_audio_file(path) {
                let song = crate::models::Song {
                    path: path.to_path_buf(),
                    title: path.file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    // TODO Add metadata
                };
                songs.push(song);
            }
        }
    }

    Ok(songs)
}

fn is_audio_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| crate::utils::SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}