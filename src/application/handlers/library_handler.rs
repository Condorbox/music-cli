use crate::application::handlers::HandlerContext;
use crate::core::events::{AppEvent, LibraryEvent, UiEvent};
use crate::modules::library::search_engine::SearchEngine;
use anyhow::Result;
use crate::modules::library::scanner;
use std::thread;

/// Handles all [`LibraryEvent`] variants.
///
/// Responsible for:
/// - Keeping the shuffle manager in sync when the library changes
/// - Executing search queries and emitting results
/// - Persisting library changes to storage
pub struct LibraryHandler {
    search_engine: SearchEngine,
}

impl LibraryHandler {
    pub fn new() -> Self {
        Self {
            search_engine: SearchEngine::new(),
        }
    }

    pub fn handle(&self, event: &LibraryEvent, ctx: &mut HandlerContext) -> Result<()> {
        match event {
            LibraryEvent::ScanCompleted { songs, .. } => {
                let len = songs.len();
                ctx.shuffle_manager.update_playlist_size(len);
                if ctx.shuffle_manager.is_enabled() {
                    ctx.shuffle_manager.initialize(len, None);
                }
                ctx.persist_state()?;
            }

            LibraryEvent::LibraryLoaded { songs } => {
                let len = songs.len();
                ctx.shuffle_manager.update_playlist_size(len);
                if ctx.shuffle_manager.is_enabled() {
                    ctx.shuffle_manager.initialize(len, None);
                }
            }

            LibraryEvent::SearchRequested { query } => {
                let results = {
                    let state = ctx.state.lock().unwrap();
                    let raw = self.search_engine.search(&state.library.songs, query);
                    self.search_engine.search_result_to_song_index(raw)
                };

                ctx.event_tx
                    .send(AppEvent::Library(LibraryEvent::SearchResults { results }))?;
            }

            LibraryEvent::ScanRequested { path } => {
                ctx.event_tx
                    .send(AppEvent::Library(LibraryEvent::ScanStarted { path: path.clone() }))?;

                let event_tx = ctx.event_tx.clone();
                let scan_path = path.clone();

                thread::spawn(move || {
                    match scanner::scan_directory(&scan_path) {
                        Ok(songs) => {
                            let count = songs.len();
                            if let Err(err) = event_tx.send(AppEvent::Library(LibraryEvent::ScanCompleted {
                                songs,
                                count,
                            })) {
                                eprintln!("Failed to send ScanCompleted event: {}", err);
                            }
                        }
                        Err(e) => {
                            let message = e.to_string();
                            if let Err(err) = event_tx.send(AppEvent::Library(LibraryEvent::ScanFailed {
                                path: scan_path.clone(),
                                message: message.clone(),
                            })) {
                                eprintln!("Failed to send ScanFailed event: {}", err);
                            }

                            if let Err(err) = event_tx.send(AppEvent::Ui(UiEvent::ShowError {
                                message: format!("Scan failed: {}", message),
                            })) {
                                eprintln!("Failed to send ShowError event: {}", err);
                            }
                        }
                    }
                });
            }

            // All other variants are handled by AppState::apply_event.
            _ => {}
        }

        Ok(())
    }
}

impl Default for LibraryHandler {
    fn default() -> Self {
        Self::new()
    }
}