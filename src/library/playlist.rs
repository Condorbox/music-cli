use std::fs;
use anyhow::Context;
use walkdir::WalkDir;
use anyhow::Result;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;

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

    let new_library = scan_directory(root, ui)?;
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

fn scan_directory(root: &std::path::Path, ui: &mut impl Ui) -> Result<Vec<Song>> {
    let mut songs = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && is_audio_file(path){
            match extract_song_metadata(path) {
                Ok(song ) => songs.push(song),
                Err(e) => {
                    ui.print_error(&format!("Warning: Failed to read metadata for {:?}: {}", path, e));

                    // Fallback to basic file info
                    songs.push(Song {
                        path: path.to_path_buf(),
                        title: path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown")
                            .to_string(),
                        artist: None,
                        album: None,
                        track_number: None,
                        duration: None,
                    });

                }
            }
        }
    }

    Ok(songs)
}

fn extract_song_metadata(path: &std::path::Path) -> Result<Song> {
    let tagged_file = Probe::open(path)
        .context("Failed to open audio file")?
        .read()
        .context("Failed to read audio file")?;

    // Get the primary tag (ID3v2 for MP3, Vorbis for FLAC/OGG, etc.)
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

    let title = tag
        .and_then(|t| t.title().map(|s| s.to_string()))
        .unwrap_or_else(|| {
            // Fallback to filename if no title tag
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    let artist = tag.and_then(|t| t.artist().map(|s| s.to_string()));

    let album = tag.and_then(|t| t.album().map(|s| s.to_string()));

    let track_number = tag.and_then(|t| t.track());

    // Get duration from audio properties
    let duration = tagged_file
        .properties()
        .duration()
        .as_secs();

    Ok(Song {
        path: path.to_path_buf(),
        title,
        artist,
        album,
        track_number,
        duration: Some(duration),
    })
}


fn is_audio_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| crate::utils::SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}