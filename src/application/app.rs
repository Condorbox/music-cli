use crate::application::state::AppState;
use crate::core::events::*;
use crate::core::traits::*;
use anyhow::Result;
use crossbeam_channel::bounded;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::core::models::RepeatMode;
use crate::modules::library::search_engine::SearchEngine;
use crate::modules::playback::shuffle_manager::ShuffleManager;
use crate::utils::volume_percent_to_amplitude;

enum NavTarget {
    /// Move to a different song at this index.
    Go(usize),
    /// Replay the current song from the beginning.
    Restart,
    /// No playback action (e.g. nothing is playing yet).
    Nothing,
}

/// Main application orchestrator
pub struct Application {
    state: Arc<Mutex<AppState>>,
    event_tx: EventSender,
    event_rx: EventReceiver,

    shuffle_manager: ShuffleManager,

    // Module references
    playback_backend: Option<Box<dyn PlaybackBackend>>,
    storage_backend: Option<Box<dyn StorageBackend>>,
    ui_renderer: Option<Box<dyn UiRenderer>>,

    // Keep track of running state
    running: Arc<Mutex<bool>>,
}

impl Application {
    pub fn new() -> Self {
        let (tx, rx) = bounded(100);

        Self {
            state: Arc::new(Mutex::new(AppState::default())),
            event_tx: tx,
            event_rx: rx,
            shuffle_manager: ShuffleManager::new(),
            playback_backend: None,
            storage_backend: None,
            ui_renderer: None,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Set the playback backend
    pub fn with_playback_backend(mut self, backend: Box<dyn PlaybackBackend>) -> Self {
        self.playback_backend = Some(backend);
        self
    }

    /// Set the storage backend
    pub fn with_storage_backend(mut self, backend: Box<dyn StorageBackend>) -> Self {
        self.storage_backend = Some(backend);
        self
    }

    /// Set the UI renderer
    pub fn with_ui_renderer(mut self, renderer: Box<dyn UiRenderer>) -> Self {
        self.ui_renderer = Some(renderer);
        self
    }

    /// Get event sender (for modules to emit events)
    pub fn event_sender(&self) -> EventSender {
        self.event_tx.clone()
    }

    /// Get current state (read-only)
    pub fn state(&self) -> AppState {
        self.state.lock().unwrap().clone()
    }

    /// Initialize the application
    pub fn init(&mut self) -> Result<()> {
        // Load state from storage
        if let Some(storage) = &self.storage_backend {
            match storage.load() {
                Ok(loaded_state) => {
                    let volume = loaded_state.config.volume;
                    let shuffle_enabled = loaded_state.config.shuffle;
                    let playlist_size = loaded_state.library.songs.len();
                    *self.state.lock().unwrap() = loaded_state;

                    // Set volume on playback backend
                    if let Some(playback) = &mut self.playback_backend {
                        playback.set_volume(volume);
                    }

                    // Initialize shuffle manager
                    self.shuffle_manager.set_enabled(shuffle_enabled);
                    if shuffle_enabled && playlist_size > 0 {
                        self.shuffle_manager.initialize(playlist_size, None);
                    }

                    // Emit library loaded event
                    let songs = self.state.lock().unwrap().library.songs.clone();
                    self.event_tx
                        .send(AppEvent::Library(LibraryEvent::LibraryLoaded { songs }))?;
                }
                Err(e) => {
                    eprintln!("Warning: Could not load state: {}", e);
                }
            }
        }

        // Initialize UI
        if let Some(ui) = &mut self.ui_renderer {
            ui.init()?;
        }

        Ok(())
    }

    /// Run the main event loop
    pub fn run(&mut self) -> Result<()> {
        *self.running.lock().unwrap() = true;

        while *self.running.lock().unwrap() {
            // Process all pending events
            self.process_events()?;

            // Poll UI for input
            if let Some(ui) = &mut self.ui_renderer {
                let ui_events = ui.poll_input()?;
                for event in ui_events {
                    self.event_tx.send(AppEvent::Ui(event))?;
                }
            }

            // Update playback position if playing (for progress bar)
            if let Some(playback) = &self.playback_backend {
                if playback.is_playing() && !playback.is_paused() {
                    // Get position from playback backend and update state
                    let position = playback.position();
                    self.state.lock().unwrap().playback.current_elapsed = position;
                }

                // Check if track finished
                if playback.has_finished() {
                    self.event_tx
                        .send(AppEvent::Playback(PlaybackEvent::TrackFinished))?;
                }
            }

            // Render UI with current state
            if let Some(ui) = &mut self.ui_renderer {
                let state = self.state.lock().unwrap();

                ui.update_state(&state);
                ui.render(&state.ui)?;
            }

            // Small sleep to prevent CPU spinning
            std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }

        Ok(())
    }

    /// Process all pending events in the queue
    fn process_events(&mut self) -> Result<()> {
        // Drain all events currently in queue
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_event(event)?;
        }
        Ok(())
    }

    /// Process events once without entering the main loop (useful for one-off commands)
    pub fn run_once(&mut self) -> Result<()> {
        self.process_events()?;
        Ok(())
    }

    /// Handle a single event
    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        // Update state based on event
        self.state.lock().unwrap().apply_event(&event);

        // Route event to appropriate handler
        match &event {
            AppEvent::Playback(pe) => self.handle_playback_event(pe)?,
            AppEvent::Library(le) => self.handle_library_event(le)?,
            AppEvent::Ui(ue) => self.handle_ui_event(ue)?,
            AppEvent::Shutdown => {
                *self.running.lock().unwrap() = false;
            }
        }

        Ok(())
    }

    fn handle_playback_event(&mut self, event: &PlaybackEvent) -> Result<()> {
        let playback = match &mut self.playback_backend {
            Some(p) => p,
            None => return Ok(()),
        };

        match event {
            PlaybackEvent::PlayRequested { song } => {
                playback.play(song)?;
                self.event_tx
                    .send(AppEvent::Playback(PlaybackEvent::Started {
                        song: song.clone(),
                    }))?;
            }

            PlaybackEvent::TrackFinished => {
                self.handle_track_finished()?;
            }

            PlaybackEvent::VolumeChanged { volume } => {
                playback.set_volume(*volume);

                // Save to storage
                if let Some(storage) = &self.storage_backend {
                    let state = self.state.lock().unwrap();
                    storage.save(&state)?;
                }
            }

            PlaybackEvent::Shuffle { enabled: _enabled } => {
                // State already updated by apply_event

                // Save to storage
                if let Some(storage) = &self.storage_backend {
                    let state = self.state.lock().unwrap();
                    storage.save(&state)?;
                }
            }

            PlaybackEvent::RepeatChanged { mode: _mode } => {
                // State already updated by apply_event

                // Save to storage
                if let Some(storage) = &self.storage_backend {
                    let state = self.state.lock().unwrap();
                    storage.save(&state)?;
                }
            }

            _ => {}
        }

        Ok(())
    }

    fn handle_track_finished(&mut self) -> Result<()> {
        let state = self.state.lock().unwrap();
        let repeat = state.config.repeat;
        let current_index = state.playback.current_index;
        let library_len = state.library.songs.len();
        drop(state);

        match repeat {
            // Repeat the same song immediately.
            RepeatMode::One => {
                if let Some(idx) = current_index {
                    let song = self
                        .state
                        .lock()
                        .unwrap()
                        .library
                        .songs
                        .get(idx)
                        .cloned();

                    if let Some(song) = song {
                        self.event_tx
                            .send(AppEvent::Playback(PlaybackEvent::PlayRequested { song }))?;
                    }
                }
            }

            // Advance to the next track; loop playlist when exhausted.
            RepeatMode::All => {
                self.advance_to_next(current_index, library_len, true)?;
            }

            // Advance to the next track; stop when the end is reached.
            RepeatMode::Off => {
                self.advance_to_next(current_index, library_len, false)?;
            }
        }

        Ok(())
    }

    fn advance_to_next(
        &mut self,
        current_index: Option<usize>,
        library_len: usize,
        loop_playlist: bool,
    ) -> Result<()> {
        let target = if self.shuffle_manager.is_enabled() {
            if self.shuffle_manager.remaining_in_pass() == 0 {
                self.shuffle_manager.initialize(library_len, current_index);
            }
            match self.shuffle_manager.next_index(current_index, loop_playlist) {
                Some(idx) => NavTarget::Go(idx),
                None => NavTarget::Restart, // shuffle exhausted, no loop —> restart current
            }
        } else {
            match current_index {
                Some(idx) => {
                    let next = idx + 1;
                    if next < library_len {
                        NavTarget::Go(next)
                    } else if loop_playlist {
                        NavTarget::Go(0)
                    } else {
                        NavTarget::Restart // at end, no loop —>restart current
                    }
                }
                None => NavTarget::Nothing,
            }
        };

        self.execute_nav(target, current_index)?;
        Ok(())
    }

    fn advance_to_prev(
        &mut self,
        current_index: Option<usize>,
        library_len: usize,
        loop_playlist: bool,
    ) -> Result<()> {
        let target = if self.shuffle_manager.is_enabled() {
            // Walk back through the existing shuffle history — no re-shuffle needed.
            match self.shuffle_manager.previous_index(current_index) {
                Some(idx) => NavTarget::Go(idx),
                None => NavTarget::Restart, // at start of history, no loop — restart current
            }
        } else {
            match current_index {
                Some(0) => {
                    if loop_playlist {
                        NavTarget::Go(library_len.saturating_sub(1)) // wrap to last
                    } else {
                        NavTarget::Restart // at first song, no loop — restart current
                    }
                }
                Some(idx) => NavTarget::Go(idx - 1),
                None => NavTarget::Nothing,
            }
        };

        self.execute_nav(target, current_index)?;
        Ok(())
    }

    fn execute_nav(&mut self, target: NavTarget, current_index: Option<usize>) -> Result<()> {
        let play_index = match target {
            NavTarget::Go(idx) => Some(idx),
            NavTarget::Restart => current_index, // replay same song from the top
            NavTarget::Nothing => None,
        };

        if let Some(idx) = play_index {
            let mut state = self.state.lock().unwrap();
            state.ui.selected_index = Some(idx);
            let song = state.library.songs.get(idx).cloned();
            drop(state);

            if let Some(song) = song {
                self.event_tx
                    .send(AppEvent::Playback(PlaybackEvent::PlayRequested { song }))?;
            }
        }

        Ok(())
    }

    fn handle_library_event(&mut self, event: &LibraryEvent) -> Result<()> {
        match event {
            LibraryEvent::ScanCompleted { songs, .. } => {
                // Update shuffle manager with new playlist size
                self.shuffle_manager.update_playlist_size(songs.len());
                if self.shuffle_manager.is_enabled() {
                    self.shuffle_manager.initialize(songs.len(), None);
                }

                // Save to storage
                if let Some(storage) = &self.storage_backend {
                    let state = self.state.lock().unwrap();
                    storage.save(&state)?;
                }
            }

            LibraryEvent::LibraryLoaded { songs } => {
                // Update shuffle manager when library is loaded
                self.shuffle_manager.update_playlist_size(songs.len());
                if self.shuffle_manager.is_enabled() {
                    self.shuffle_manager.initialize(songs.len(), None);
                }
            }

            LibraryEvent::SearchRequested { query } => {
                let state = self.state.lock().unwrap();

                // Use fuzzy search engine instead of simple contains
                let search_engine = SearchEngine::new();
                let search_results = search_engine.search(&state.library.songs, query);

                // Convert SearchResult to (index, Song) tuples
                let results: Vec<(usize, crate::core::models::Song)> = search_engine.search_result_to_song_index(search_results);

                drop(state);

                self.event_tx
                    .send(AppEvent::Library(LibraryEvent::SearchResults { results }))?;
            }

            _ => {}
        }

        Ok(())
    }

    fn handle_ui_event(&mut self, event: &UiEvent) -> Result<()> {
        match event {
            UiEvent::PlaySelectedRequested => {
                let state = self.state.lock().unwrap();
                if let Some(index) = state.ui.selected_index {
                    if let Some(song) = state.library.songs.get(index) {
                        let song = song.clone();
                        drop(state);

                        self.event_tx
                            .send(AppEvent::Playback(PlaybackEvent::PlayRequested { song }))?;
                    }
                }
            }

            UiEvent::TogglePauseRequested => {
                if let Some(playback) = &mut self.playback_backend {
                    if playback.is_paused() {
                        playback.resume();
                        self.event_tx
                            .send(AppEvent::Playback(PlaybackEvent::Resumed))?;
                    } else if playback.is_playing() {
                        playback.pause();
                        self.event_tx
                            .send(AppEvent::Playback(PlaybackEvent::Paused))?;
                    }
                }
            }

            UiEvent::NextTrackRequested => {
                let state = self.state.lock().unwrap();
                let current_index = state.ui.selected_index;
                let library_len = state.library.songs.len();
                // RepeatMode::One does not loop on manual nav — user explicitly asked to move.
                let loop_playlist = state.config.repeat == RepeatMode::All;
                drop(state);

                // Re-initialize shuffle queue if exhausted (new pass).
                if self.shuffle_manager.is_enabled() && self.shuffle_manager.remaining_in_pass() == 0 {
                    self.shuffle_manager.initialize(library_len, current_index);
                }

                self.advance_to_next(current_index, library_len, loop_playlist)?;
            }

            UiEvent::PreviousTrackRequested => {
                let state = self.state.lock().unwrap();
                let current_index = state.ui.selected_index;
                let library_len = state.library.songs.len();
                // RepeatMode::One does not loop on manual nav — user explicitly asked to move.
                let loop_playlist = state.config.repeat == RepeatMode::All;
                drop(state);

                self.advance_to_prev(current_index, library_len, loop_playlist)?;
            }

            UiEvent::VolumeChangeRequested { volume } => {
                let volume_f32 = volume_percent_to_amplitude(*volume);

                self.event_tx
                    .send(AppEvent::Playback(PlaybackEvent::VolumeChanged {
                        volume: volume_f32
                    }))?;

                self.event_tx
                    .send(AppEvent::Ui(UiEvent::ShowMessage {
                        message: format!("Volume set to {}%", volume)
                    }))?;
            }

            UiEvent::PathChangeRequested { path } => {
                if !path.is_dir() {
                    self.event_tx
                        .send(AppEvent::Ui(UiEvent::ShowError {
                            message: "Invalid directory path".to_string()
                        }))?;
                    return Ok(());
                }

                // Update config
                let mut state = self.state.lock().unwrap();
                state.config.root_path = Some(path.clone());
                drop(state);

                // Save to storage
                if let Some(storage) = &self.storage_backend {
                    let state = self.state.lock().unwrap();
                    storage.save(&state)?;
                }

                self.event_tx
                    .send(AppEvent::Ui(UiEvent::ShowMessage {
                        message: "Music path updated. Run refresh to scan.".to_string()
                    }))?;
            }

            // Handle search toggle
            UiEvent::SearchToggled { active } => {
                // State update happens in apply_event
                // Just emit confirmation message
                if !active {
                    self.event_tx
                        .send(AppEvent::Ui(UiEvent::ShowMessage {
                            message: "Search cleared".to_string()
                        }))?;
                }
            }

            // Handle search query change
            UiEvent::SearchQueryChanged { query } => {
                // Trigger actual search through Library event
                self.event_tx
                    .send(AppEvent::Library(LibraryEvent::SearchRequested {
                        query: query.clone()
                    }))?;
            }

            UiEvent::ShuffleToggled { shuffle_enabled } => {
                let new_state = !shuffle_enabled;

                self.shuffle_manager.set_enabled(new_state);

                // If enabling shuffle, initialize it with current state
                if new_state {
                    let state = self.state.lock().unwrap();
                    let current_index = state.ui.selected_index;
                    let playlist_size = state.library.songs.len();
                    drop(state);

                    self.shuffle_manager.initialize(playlist_size, current_index);
                }

                // Send event to update config and save
                self.event_tx.send(AppEvent::Playback(PlaybackEvent::Shuffle {
                    enabled: new_state
                }))?;
            }

            UiEvent::ShuffleSet { enabled } => {
                self.shuffle_manager.set_enabled(*enabled);

                // If enabling shuffle, initialize it with current state
                if *enabled {
                    let state = self.state.lock().unwrap();
                    let current_index = state.ui.selected_index;
                    let playlist_size = state.library.songs.len();
                    drop(state);

                    self.shuffle_manager.initialize(playlist_size, current_index);
                }

                self.event_tx.send(AppEvent::Playback(PlaybackEvent::Shuffle {
                    enabled: *enabled
                }))?;
            }

            UiEvent::RepeatChangeRequested { mode } => {
                self.event_tx
                    .send(AppEvent::Playback(PlaybackEvent::RepeatChanged { mode: *mode }))?;
            }

            UiEvent::QuitRequested => {
                self.event_tx.send(AppEvent::Shutdown)?;
            }

            _ => {}
        }

        Ok(())
    }

    /// Cleanup resources
    pub fn cleanup(&mut self) -> Result<()> {
        // Save state
        if let Some(storage) = &self.storage_backend {
            let state = self.state.lock().unwrap();
            storage.save(&state)?;
        }

        // Cleanup UI
        if let Some(ui) = &mut self.ui_renderer {
            ui.cleanup()?;
        }

        Ok(())
    }
}
