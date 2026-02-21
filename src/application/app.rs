use crate::application::state::AppState;
use crate::core::events::*;
use crate::core::traits::*;
use anyhow::Result;
use crossbeam_channel::bounded;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::application::handlers::HandlerContext;
use crate::application::handlers::library_handler::LibraryHandler;
use crate::application::handlers::playback_handler::PlaybackHandler;
use crate::application::handlers::ui_handler::UiHandler;
use crate::modules::playback::shuffle_manager::ShuffleManager;

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

    // Handlers
    playback_handler: PlaybackHandler,
    library_handler: LibraryHandler,
    ui_handler: UiHandler,
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
            playback_handler: PlaybackHandler,
            library_handler: LibraryHandler::new(),
            ui_handler: UiHandler,
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
            self.process_events()?;
            self.poll_ui_input()?;
            self.tick_playback()?;
            self.render()?;

            // Small sleep to prevent CPU spinning
            std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }

        Ok(())
    }

    /// Process events once
    pub fn run_once(&mut self) -> Result<()> {
        self.process_events()
    }

    /// Cleanup resources and persist final state
    pub fn cleanup(&mut self) -> Result<()> {
        if let Some(storage) = &self.storage_backend {
            let state = self.state.lock().unwrap();
            storage.save(&state)?;
        }

        if let Some(ui) = &mut self.ui_renderer {
            ui.cleanup()?;
        }

        Ok(())
    }

    fn process_events(&mut self) -> Result<()> {
        while let Ok(event) = self.event_rx.try_recv() {
            self.dispatch(event)?;
        }
        Ok(())
    }

    fn poll_ui_input(&mut self) -> Result<()> {
        if let Some(ui) = &mut self.ui_renderer {
            for event in ui.poll_input()? {
                self.event_tx.send(AppEvent::Ui(event))?;
            }
        }
        Ok(())
    }

    fn tick_playback(&mut self) -> Result<()> {
        if let Some(playback) = &self.playback_backend {
            if playback.is_playing() && !playback.is_paused() {
                let position = playback.position();
                self.state.lock().unwrap().playback.current_elapsed = position;
            }

            if playback.has_finished() {
                self.event_tx
                    .send(AppEvent::Playback(PlaybackEvent::TrackFinished))?;
            }
        }
        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        if let Some(ui) = &mut self.ui_renderer {
            let state = self.state.lock().unwrap();
            ui.update_state(&state);
            ui.render(&state.ui)?;
        }
        Ok(())
    }

    /// Apply state update then delegate side effects to the appropriate handler
    fn dispatch(&mut self, event: AppEvent) -> Result<()> {
        self.state.lock().unwrap().apply_event(&event);

        let mut ctx = HandlerContext {
            state: &self.state,
            event_tx: &self.event_tx,
            playback: &mut self.playback_backend,
            storage: &self.storage_backend,
            shuffle_manager: &mut self.shuffle_manager,
        };

        match &event {
            AppEvent::Playback(pe) => self.playback_handler.handle(pe, &mut ctx)?,
            AppEvent::Library(le) => self.library_handler.handle(le, &mut ctx)?,
            AppEvent::Ui(ue) => self.ui_handler.handle(ue, &mut ctx)?,
            AppEvent::Shutdown => *self.running.lock().unwrap() = false,
        }

        Ok(())
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}