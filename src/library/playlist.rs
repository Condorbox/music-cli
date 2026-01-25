use std::fs;
use anyhow::Context;
use walkdir::WalkDir;
use anyhow::Result;

use crate::library::store::StoreManager;
use crate::models::Song;
use crate::utils::APP_NAME;

use crate::player::audio;
use crate::ui::Ui;

pub fn handle_set_path(path_str: String, store: &StoreManager, ui: &mut impl Ui) -> Result<()> {
    let path = std::path::PathBuf::from(&path_str);

    if !path.exists() || !path.is_dir() {
        anyhow::bail!("The path provided is not a valid directory.");
    }

    let mut state = store.load()?;
    state.config.root_path = Some(fs::canonicalize(path)?); // Store absolute path

    store.save(&state)?;
    ui.print_message(&format!("Music path updated to: {:?}", state.config.root_path.unwrap()));

    handle_refresh(store, ui)
}

// TODO Maybe add an optional customPath
pub fn handle_playlist(store: &StoreManager, ui: &mut impl Ui) -> Result<()> {
    let state = store.load()?;
    if state.library.is_empty() {
        anyhow::bail!("Library is empty. Run '{} refresh' or set a path.", APP_NAME);
    }

    audio::play_playlist(state.library, ui).expect("Error playing the playlist");

    Ok(())
}

pub fn handle_refresh(store: &StoreManager, ui: &mut impl Ui) -> Result<()> {
    let mut state = store.load()?;
    let root = state.config.root_path.as_ref()
        .with_context(|| {
            format!("No music path set. Run '{} path <DIR>' first.", APP_NAME)
        })?;

    ui.print_message(&format!("Scanning {:?}...", root));

    let new_library = scan_directory(root)?;
    let count = new_library.len();

    state.library = new_library;
    store.save(&state)?;

    ui.print_message(&format!("Refresh complete. Found {} songs.", count));
    Ok(())
}

pub fn handle_list(store: &StoreManager,ui: &mut impl Ui) -> Result<()> {
    let state = store.load()?;

    let songs: Vec<Song> = state.library;
    let total_songs = songs.len();

    if songs.is_empty() {
        anyhow::bail!("Library is empty. Run '{} refresh' or set a path.", APP_NAME);
    }

    for (index, song) in songs.iter().enumerate() {
        ui.print_message(&format!("\n[{}/{}] {}", index + 1, total_songs, song.title));
    }

    Ok(())
}

// TODO Maybe also select by title
pub fn handle_select(index: usize, store: &StoreManager, ui: &mut impl Ui) -> Result<()> {
    let state = store.load()?;

    if state.library.is_empty() {
        anyhow::bail!("Library is empty. Run '{} refresh' or set a path.", APP_NAME);
    }

    let song = state.library.get(index)
        .with_context(|| {
            format!(
                "Invalid index {}. Library has {} songs (0-{}).",
                index,
                state.library.len(),
                state.library.len() - 1
            )
        })?;

    audio::play_song(song, ui)?;

    Ok(())
}

pub fn handle_search(query: String, store: &StoreManager, ui: &mut impl Ui) -> Result<()> {
    let state = store.load()?;

    if state.library.is_empty() {
        anyhow::bail!("Library is empty. Run '{} refresh' or set a path.", APP_NAME);
    }

    let matches = StoreManager::search_library(&state.library, &query);

    if matches.is_empty() {
        ui.print_message(&format!("No songs found matching: '{}'", query));
        return Ok(());
    }

    ui.print_message(&format!("Found {} matches:", matches.len()));
    for (index, song) in matches {
        ui.print_message(&format!("[{}] {}", index, song));
    }

    Ok(())
}
fn scan_directory(root: &std::path::Path) -> Result<Vec<Song>> {
    let songs = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && is_audio_file(e.path()))
        .map(|entry| Song::from_path(entry.path()))
        .collect();

    Ok(songs)
}

fn is_audio_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| crate::utils::SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}