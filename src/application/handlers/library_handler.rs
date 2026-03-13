use std::sync::Arc;
use crate::application::handlers::HandlerContext;
use crate::core::events::{AppEvent, LibraryEvent, UiEvent};
use crate::modules::library::search_engine::SearchEngine;
use anyhow::Result;
use crate::modules::library::scanner;
use std::thread;
use crate::modules::library::sorter::sort_songs;

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

                // Stop the audio engine — the playing index is now stale
                if let Some(playback) = ctx.playback.as_mut() {
                    playback.stop();
                }

                // Re-anchor the shuffle queue to the new library size (position 0).
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

            LibraryEvent::SortRequested { field } => {
                let (new_selected_index, new_current_index) = {
                    let mut state = ctx.state.lock().unwrap();

                    // Record which songs are selected/playing so we can re-anchor after sort.
                    let selected_path = state.ui.selected_index
                        .and_then(|i| state.library.songs.get(i))
                        .map(|s| s.path.clone());
                    let current_path = state.playback.current_index
                        .and_then(|i| state.library.songs.get(i))
                        .map(|s| s.path.clone());

                    // Sort into a new vec and replace the Arc.
                    let sorted: Vec<_> = sort_songs(&state.library.songs, *field)
                        .into_iter()
                        .cloned()
                        .collect();
                    state.library.songs = Arc::new(sorted);

                    // Find re-anchored positions in the new order by path.
                    let new_selected = selected_path
                        .and_then(|p| state.library.songs.iter().position(|s| s.path == p));
                    let new_current = current_path
                        .and_then(|p| state.library.songs.iter().position(|s| s.path == p));

                    (new_selected, new_current)
                };

                ctx.event_tx.send(AppEvent::Library(LibraryEvent::SortChanged {
                    field: *field,
                    new_selected_index,
                    new_current_index,
                }))?;
            }

            // All other variants are handled by AppState::apply_event.
            LibraryEvent::ScanStarted { .. }
            | LibraryEvent::ScanProgress { .. }
            | LibraryEvent::ScanFailed { .. }
            | LibraryEvent::SearchResults { .. }
            | LibraryEvent::SortChanged { .. } => {}
        }

        Ok(())
    }
}

impl Default for LibraryHandler {
    fn default() -> Self {
        Self::new()
    }
}