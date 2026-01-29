use crate::application::state::AppState;
use crate::core::events::*;
use crate::core::traits::*;
use anyhow::Result;
use crossbeam_channel::bounded;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Main application orchestrator
pub struct Application {
    state: Arc<Mutex<AppState>>,
    event_tx: EventSender,
    event_rx: EventReceiver,

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
                    *self.state.lock().unwrap() = loaded_state;

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

            // Check playback state
            if let Some(playback) = &mut self.playback_backend {
                if playback.has_finished() {
                    self.event_tx
                        .send(AppEvent::Playback(PlaybackEvent::TrackFinished))?;
                }
            }

            // Render UI with current state
            if let Some(ui) = &mut self.ui_renderer {
                let state = self.state.lock().unwrap();

                // Update TuiRenderer with full app state
                if let Some(tui) = ui
                    .as_any()
                    .downcast_mut::<crate::modules::ui::tui::renderer::TuiRenderer>()
                {
                    tui.update_from_app_state(&state);
                }

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
                // Auto-advance logic (if in playlist mode)
                let state = self.state.lock().unwrap();
                if let Some(current_idx) = state.playback.current_index {
                    let next_idx = current_idx + 1;
                    if next_idx < state.playback.playlist.len() {
                        let next_song = state.playback.playlist[next_idx].clone();
                        drop(state); // Release lock before emitting event

                        // Update playlist index
                        self.state.lock().unwrap().playback.current_index = Some(next_idx);

                        self.event_tx
                            .send(AppEvent::Playback(PlaybackEvent::PlayRequested {
                                song: next_song,
                            }))?;
                    }
                }
            }

            _ => {}
        }

        Ok(())
    }

    fn handle_library_event(&mut self, event: &LibraryEvent) -> Result<()> {
        match event {
            LibraryEvent::ScanCompleted { .. } => {
                // Save to storage
                if let Some(storage) = &self.storage_backend {
                    let state = self.state.lock().unwrap();
                    storage.save(&state)?;
                }
            }

            LibraryEvent::SearchRequested { query } => {
                let state = self.state.lock().unwrap();
                let results: Vec<(usize, crate::core::models::Song)> = state
                    .library
                    .songs
                    .iter()
                    .enumerate()
                    .filter(|(_, song)| song.matches_query(query))
                    .map(|(i, song)| (i, song.clone()))
                    .collect();

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
                let mut state = self.state.lock().unwrap();
                if let Some(index) = state.ui.selected_index {
                    let next = (index + 1).min(state.library.songs.len().saturating_sub(1));
                    state.ui.selected_index = Some(next);

                    if let Some(song) = state.library.songs.get(next).cloned() {
                        drop(state);
                        self.event_tx
                            .send(AppEvent::Playback(PlaybackEvent::PlayRequested { song }))?;
                    }
                }
            }

            UiEvent::PreviousTrackRequested => {
                let mut state = self.state.lock().unwrap();
                if let Some(index) = state.ui.selected_index {
                    let prev = index.saturating_sub(1);
                    state.ui.selected_index = Some(prev);

                    if let Some(song) = state.library.songs.get(prev).cloned() {
                        drop(state);
                        self.event_tx
                            .send(AppEvent::Playback(PlaybackEvent::PlayRequested { song }))?;
                    }
                }
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
