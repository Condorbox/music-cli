use crate::core::models::Song;
use crate::core::events::*;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Complete application state (single source of truth)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub config: ConfigState,
    pub library: LibraryState,
    pub playback: PlaybackState,

    #[serde(skip)]
    pub ui: UiState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigState {
    pub root_path: Option<PathBuf>,
    pub volume: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryState {
    pub songs: Vec<Song>,

    #[serde(skip)]
    pub is_scanning: bool,

    #[serde(skip)]
    pub last_scan_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    #[serde(skip)]
    pub current_song: Option<Song>,

    #[serde(skip)]
    pub is_playing: bool,

    #[serde(skip)]
    pub is_paused: bool,

    pub volume: f32,

    #[serde(skip)]
    pub playlist: Vec<Song>,

    #[serde(skip)]
    pub current_index: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub selected_index: Option<usize>,
    pub status_message: String,
    pub error_message: Option<String>,

    // Search state
    pub search_active: bool,
    pub search_query: String,
    pub search_results: Vec<(usize, Song)>, // (original_index, song)
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            selected_index: None,
            status_message: "".to_string(),
            error_message: None,
            search_active: false,
            search_query: String::new(),
            search_results: Vec::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: ConfigState {
                root_path: None,
                volume: 1.0,
            },
            library: LibraryState {
                songs: Vec::new(),
                is_scanning: false,
                last_scan_path: None,
            },
            playback: PlaybackState {
                current_song: None,
                is_playing: false,
                is_paused: false,
                volume: 1.0,
                playlist: Vec::new(),
                current_index: None,
            },
            ui: UiState {
                selected_index: None,
                status_message: "Welcome".to_string(),
                error_message: None,
                search_active: false,
                search_query: String::new(),
                search_results: Vec::new(),
            },
        }
    }
}

impl AppState {
    /// Update state based on an event
    pub fn apply_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::Playback(pe) => match pe {
                PlaybackEvent::Started { song } => {
                    self.playback.current_song = Some(song.clone());
                    self.playback.is_playing = true;
                    self.playback.is_paused = false;
                    self.ui.status_message = format!("Playing: {}", song.title);
                    self.ui.error_message = None;
                }
                PlaybackEvent::Paused => {
                    self.playback.is_paused = true;
                    self.ui.status_message = "Paused".to_string();
                }
                PlaybackEvent::Resumed => {
                    self.playback.is_paused = false;
                    if let Some(song) = &self.playback.current_song {
                        self.ui.status_message = format!("Playing: {}", song.title);
                    }
                }
                PlaybackEvent::Stopped => {
                    self.playback.is_playing = false;
                    self.playback.is_paused = false;
                    self.playback.current_song = None;
                    self.ui.status_message = "Stopped".to_string();
                }
                PlaybackEvent::TrackFinished => {
                    self.playback.is_playing = false;
                    // Don't clear current_song - might still want to display it
                }
                PlaybackEvent::VolumeChanged { volume } => {
                    self.playback.volume = *volume;
                    self.config.volume = *volume;
                }
                PlaybackEvent::Error { message } => {
                    self.ui.error_message = Some(message.clone());
                }
                _ => {}
            },

            AppEvent::Library(le) => match le {
                LibraryEvent::ScanStarted { path } => {
                    self.library.is_scanning = true;
                    self.library.last_scan_path = Some(path.clone());
                    self.ui.status_message = format!("Scanning {:?}...", path);
                    self.ui.error_message = None;
                }
                LibraryEvent::ScanProgress { found } => {
                    self.ui.status_message = format!("Scanning... found {} songs", found);
                }
                LibraryEvent::ScanCompleted { songs, count } => {
                    self.library.songs = songs.clone();
                    self.library.is_scanning = false;
                    self.ui.status_message = format!("Found {} songs", count);

                    // Auto-select first song if nothing selected
                    if self.ui.selected_index.is_none() && !songs.is_empty() {
                        self.ui.selected_index = Some(0);
                    }
                }
                LibraryEvent::LibraryLoaded { songs } => {
                    self.library.songs = songs.clone();
                    if self.ui.selected_index.is_none() && !songs.is_empty() {
                        self.ui.selected_index = Some(0);
                    }
                }
                LibraryEvent::SearchResults { results } => {
                    self.ui.search_results = results.clone();

                    if results.is_empty() {
                        self.ui.status_message = "No results found".to_string();
                    } else {
                        self.ui.status_message = format!("Found {} matches", results.len());

                        // Auto-select first result
                        if !results.is_empty() {
                            self.ui.selected_index = Some(results[0].0);
                        }
                    }
                }
                _ => {}
            },

            AppEvent::Ui(ue) => match ue {
                UiEvent::SelectionChanged { index } => {
                    self.ui.selected_index = Some(*index);
                }
                UiEvent::ShowMessage { message } => {
                    self.ui.status_message = message.clone();
                    self.ui.error_message = None;
                }
                UiEvent::ShowError { message } => {
                    self.ui.error_message = Some(message.clone());
                }
                UiEvent::SearchToggled { active } => {
                    self.ui.search_active = *active;

                    if !active {
                        // Clear search when toggled off
                        self.ui.search_query.clear();
                        self.ui.search_results.clear();
                        self.ui.status_message = "Search cleared".to_string();

                        // Reset to first song
                        if !self.library.songs.is_empty() {
                            self.ui.selected_index = Some(0);
                        }
                    } else {
                        self.ui.status_message = "Search mode active".to_string();
                    }
                }
                UiEvent::SearchQueryChanged { query } => {
                    self.ui.search_query = query.clone();
                    // Note: Actual search is triggered by LibraryEvent::SearchRequested
                }
                _ => {}
            },

            AppEvent::Shutdown => {}
        }
    }
}