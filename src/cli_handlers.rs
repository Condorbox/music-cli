use crate::application::app::Application;
use crate::core::events::*;
use crate::core::models::Song;
use crate::modules::library::scanner;
use crate::modules::playback::rodio_backend::RodioBackend;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::modules::ui::tui::renderer::TuiRenderer;
use crate::utils::{amplitude_to_volume, volume_percent_to_amplitude, APP_NAME};
use anyhow::Result;
use std::path::PathBuf;
use crate::core::traits::{PlaybackBackend, StorageBackend};

// TODO Maye add a command pattern

pub fn handle_browse() -> Result<()> {
    let storage = JsonStorageBackend::new()?;
    let state = storage.load()?;

    if state.library.songs.is_empty() {
        let ui = TerminalRenderer::new();
        ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
        return Ok(());
    }

    let mut tui_renderer = TuiRenderer::new();
    tui_renderer.set_songs(state.library.songs.clone());

    let mut app = Application::new()
        .with_playback_backend(Box::new(RodioBackend::new()?))
        .with_storage_backend(Box::new(storage))
        .with_ui_renderer(Box::new(tui_renderer));

    app.init()?;
    app.run()?;
    app.cleanup()?;

    Ok(())
}

pub fn handle_play(file: PathBuf) -> Result<()> {
    let song = Song::from_path(&file);
    let ui = TerminalRenderer::new();

    ui.print_message(&format!("Playing: {}", song.title));

    let mut backend = RodioBackend::new()?;
    backend.play(&song)?;

    ui.print_message("Press Ctrl+C to stop");
    while backend.is_playing() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    ui.print_message("✓ Playback finished");

    Ok(())
}

// TODO Maybe refresh too
pub fn handle_path(directory: PathBuf) -> Result<()> {
    let storage = JsonStorageBackend::new()?;
    let mut state = storage.load()?;
    let ui = TerminalRenderer::new();

    let path = directory.canonicalize()?;
    if !path.is_dir() {
        anyhow::bail!("The path provided is not a valid directory.");
    }

    state.config.root_path = Some(path.clone());
    storage.save(&state)?;

    ui.print_message(&format!("Music path updated to: {:?}", path));
    ui.print_message(&format!("Run '{} refresh' to scan for music files.", APP_NAME));

    // handle_refresh()?;

    Ok(())
}

pub fn handle_refresh() -> Result<()> {
    let storage = JsonStorageBackend::new()?;
    let mut state = storage.load()?;
    let ui = TerminalRenderer::new();

    let root_path = state.config.root_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!(
            "No music path set. Run '{} path <DIR>' first.", APP_NAME
        ))?
        .clone();

    ui.print_message(&format!("Scanning {:?}...", root_path));

    let songs = scanner::scan_directory(&root_path)?;
    let count = songs.len();

    state.library.songs = songs;
    storage.save(&state)?;

    ui.print_message(&format!("✓ Refresh complete. Found {} songs.", count));

    Ok(())
}

pub fn handle_playlist() -> Result<()> {
    let storage = JsonStorageBackend::new()?;
    let state = storage.load()?;
    let ui = TerminalRenderer::new();

    if state.library.songs.is_empty() {
        ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
        return Ok(());
    }

    ui.print_message(&format!("Queueing {} songs...\n", state.library.songs.len()));

    let mut app = Application::new()
        .with_playback_backend(Box::new(RodioBackend::new()?))
        .with_storage_backend(Box::new(storage))
        .with_ui_renderer(Box::new(TerminalRenderer::new()));

    // Set up playlist in state
    {
        let mut app_state = app.state();
        app_state.playback.playlist = state.library.songs.clone();
        app_state.playback.current_index = Some(0);
        app_state.library.songs = state.library.songs.clone();
    }

    app.init()?;

    // Start playing first song
    let first_song = state.library.songs[0].clone();
    app.event_sender().send(AppEvent::Playback(PlaybackEvent::PlayRequested {
        song: first_song
    }))?;

    app.run()?;
    app.cleanup()?;

    Ok(())
}

pub fn handle_list() -> Result<()> {
    let storage = JsonStorageBackend::new()?;
    let state = storage.load()?;
    let ui = TerminalRenderer::new();

    if state.library.songs.is_empty() {
        ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
        return Ok(());
    }

    ui.print_song_list(&state.library.songs);

    Ok(())
}

pub fn handle_select(index: usize) -> Result<()> {
    let storage = JsonStorageBackend::new()?;
    let state = storage.load()?;
    let ui = TerminalRenderer::new();

    if state.library.songs.is_empty() {
        ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
        return Ok(());
    }

    let song = state.library.songs.get(index)
        .ok_or_else(|| anyhow::anyhow!(
            "Invalid index {}. Library has {} songs (0-{}).",
            index,
            state.library.songs.len(),
            state.library.songs.len() - 1
        ))?;

    ui.print_message(&format!("Playing: {}", song.title));

    let mut backend = RodioBackend::new()?;
    backend.play(song)?;

    ui.print_message("Press Ctrl+C to stop");
    while backend.is_playing() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    ui.print_message("✓ Playback finished");

    Ok(())
}

pub fn handle_search(query: String) -> Result<()> {
    let storage = JsonStorageBackend::new()?;
    let state = storage.load()?;
    let ui = TerminalRenderer::new();

    if state.library.songs.is_empty() {
        ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
        return Ok(());
    }

    let matches: Vec<_> = state.library.songs
        .iter()
        .enumerate()
        .filter(|(_, song)| song.matches_query(&query))
        .map(|(i, song)| (i, song.clone()))
        .collect();

    ui.print_search_results(&query, &matches);

    Ok(())
}

pub fn handle_volume(volume: Option<u8>) -> Result<()> {
    let storage = JsonStorageBackend::new()?;
    let state = storage.load()?;
    let ui = TerminalRenderer::new();

    match volume {
        Some(vol) => {
            let volume_f32 = volume_percent_to_amplitude(vol);

            let mut app = Application::new()
                .with_playback_backend(Box::new(RodioBackend::new()?))
                .with_storage_backend(Box::new(storage))
                .with_ui_renderer(Box::new(ui));

            app.init()?;

            // Send volume change event
            app.event_sender().send(AppEvent::Playback(PlaybackEvent::VolumeChanged {
                volume: volume_f32,
            }))?;

            app.run_once()?;

            app.cleanup()?;

            let ui = TerminalRenderer::new();
            ui.print_message(&format!("Volume set to: {}%", vol));
        }
        None => {
            let current_percent = amplitude_to_volume(state.config.volume);
            ui.print_message(&format!("Current volume: {}%", current_percent));
        }
    }

    Ok(())
}