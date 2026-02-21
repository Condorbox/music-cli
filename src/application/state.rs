use crate::core::models::{RepeatMode, Song};
use crate::core::events::*;
use std::path::PathBuf;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Complete application state (single source of truth)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    #[serde(default)]
    pub config: ConfigState,

    #[serde(default)]
    pub library: LibraryState,

    #[serde(default)]
    pub playback: PlaybackState,

    #[serde(skip)]
    pub ui: UiState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigState {
    #[serde(default)]
    pub root_path: Option<PathBuf>,

    #[serde(default = "default_volume")]
    pub volume: f32,

    #[serde(default)]
    pub shuffle: bool,

    #[serde(default)]
    pub repeat: RepeatMode,
}

fn default_volume() -> f32 {
    1.0
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

    #[serde(skip)]
    pub current_elapsed: Duration,
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

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            root_path: None,
            volume: default_volume(),
            shuffle: false,
            repeat: Default::default(),
        }
    }
}

impl Default for LibraryState {
    fn default() -> Self {
        Self {
            songs: Vec::new(),
            is_scanning: false,
            last_scan_path: None,
        }
    }
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            current_song: None,
            is_playing: false,
            is_paused: false,
            volume: default_volume(),
            playlist: Vec::new(),
            current_index: None,
            current_elapsed: Duration::from_secs(0),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: ConfigState {
                root_path: None,
                volume: 1.0,
                shuffle: false,
                repeat: RepeatMode::default(),
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
                current_elapsed: Duration::from_secs(0),
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
                    self.playback.current_index = self.ui.selected_index;
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
                PlaybackEvent::Shuffle { enabled} => {
                    self.config.shuffle = *enabled;
                }
                PlaybackEvent::RepeatChanged { mode } => {
                    self.config.repeat = *mode;
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

                        if let Some(playing_index) = self.playback.current_index {
                            // If something is playing, jump to that song
                            self.ui.selected_index = Some(playing_index);
                        } else if let Some(selected) = self.ui.selected_index {
                            // If nothing is playing, but we had a selection, keep it
                            self.ui.selected_index = Some(selected);
                        } else if !self.library.songs.is_empty() {
                            // Go to first song
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{RepeatMode, Song};
    use std::path::PathBuf;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_song(title: &str) -> Song {
        Song {
            path: PathBuf::from(format!("{}.mp3", title)),
            title: title.to_owned(),
            artist: Some("Test Artist".to_owned()),
            album: Some("Test Album".to_owned()),
            track_number: None,
            duration: None,
            search_key: title.to_lowercase(),
        }
    }

    fn state_with_songs(n: usize) -> AppState {
        let mut state = AppState::default();
        state.library.songs = (0..n).map(|i| make_song(&format!("Song {}", i))).collect();
        state
    }

    fn apply(state: &mut AppState, event: AppEvent) {
        state.apply_event(&event);
    }

    // ── PlaybackEvent::Started ────────────────────────────────────────────────

    #[test]
    fn started_sets_current_song_and_playing_flags() {
        let mut state = AppState::default();
        state.ui.selected_index = Some(2);
        let song = make_song("Test Song");

        apply(&mut state, AppEvent::Playback(PlaybackEvent::Started { song: song.clone() }));

        assert_eq!(state.playback.current_song.as_ref().unwrap().title, song.title);
        assert!(state.playback.is_playing);
        assert!(!state.playback.is_paused);
    }

    #[test]
    fn started_captures_selected_index_as_current_index() {
        let mut state = AppState::default();
        state.ui.selected_index = Some(3);

        apply(&mut state, AppEvent::Playback(PlaybackEvent::Started { song: make_song("X") }));

        assert_eq!(state.playback.current_index, Some(3));
    }

    #[test]
    fn started_updates_status_and_clears_error() {
        let mut state = AppState::default();
        state.ui.error_message = Some("old error".to_owned());

        apply(&mut state, AppEvent::Playback(PlaybackEvent::Started { song: make_song("My Song") }));

        assert!(state.ui.status_message.contains("My Song"));
        assert!(state.ui.error_message.is_none());
    }

    // ── PlaybackEvent::Paused ─────────────────────────────────────────────────

    #[test]
    fn paused_sets_is_paused_and_status() {
        let mut state = AppState::default();
        state.playback.is_playing = true;

        apply(&mut state, AppEvent::Playback(PlaybackEvent::Paused));

        assert!(state.playback.is_paused);
        assert_eq!(state.ui.status_message, "Paused");
    }

    // ── PlaybackEvent::Resumed ────────────────────────────────────────────────

    #[test]
    fn resumed_clears_is_paused() {
        let mut state = AppState::default();
        state.playback.is_paused = true;
        state.playback.current_song = Some(make_song("Playing Song"));

        apply(&mut state, AppEvent::Playback(PlaybackEvent::Resumed));

        assert!(!state.playback.is_paused);
        assert!(state.ui.status_message.contains("Playing Song"));
    }

    #[test]
    fn resumed_without_current_song_still_clears_paused() {
        let mut state = AppState::default();
        state.playback.is_paused = true;

        apply(&mut state, AppEvent::Playback(PlaybackEvent::Resumed));

        assert!(!state.playback.is_paused);
    }

    // ── PlaybackEvent::Stopped ────────────────────────────────────────────────

    #[test]
    fn stopped_clears_all_playback_state() {
        let mut state = AppState::default();
        state.playback.current_song = Some(make_song("A Song"));
        state.playback.is_playing = true;
        state.playback.is_paused = true;

        apply(&mut state, AppEvent::Playback(PlaybackEvent::Stopped));

        assert!(!state.playback.is_playing);
        assert!(!state.playback.is_paused);
        assert!(state.playback.current_song.is_none());
        assert_eq!(state.ui.status_message, "Stopped");
    }

    // ── PlaybackEvent::TrackFinished ──────────────────────────────────────────

    #[test]
    fn track_finished_sets_is_playing_false_but_preserves_current_song() {
        let mut state = AppState::default();
        let song = make_song("Finishing Song");
        state.playback.current_song = Some(song.clone());
        state.playback.is_playing = true;

        apply(&mut state, AppEvent::Playback(PlaybackEvent::TrackFinished));

        assert!(!state.playback.is_playing, "is_playing must be false after track finishes");
        assert!(
            state.playback.current_song.is_some(),
            "current_song must be preserved so the handler can replay it (RepeatMode::One)"
        );
    }

    // ── PlaybackEvent::VolumeChanged ──────────────────────────────────────────

    #[test]
    fn volume_changed_updates_both_config_and_playback_volume() {
        let mut state = AppState::default();

        apply(&mut state, AppEvent::Playback(PlaybackEvent::VolumeChanged { volume: 0.42 }));

        assert!((state.config.volume - 0.42).abs() < f32::EPSILON);
        assert!((state.playback.volume - 0.42).abs() < f32::EPSILON,
                "playback.volume must also be updated");
    }

    // ── PlaybackEvent::Shuffle ────────────────────────────────────────────────

    #[test]
    fn shuffle_event_updates_config_shuffle() {
        let mut state = AppState::default();
        assert!(!state.config.shuffle);

        apply(&mut state, AppEvent::Playback(PlaybackEvent::Shuffle { enabled: true }));
        assert!(state.config.shuffle);

        apply(&mut state, AppEvent::Playback(PlaybackEvent::Shuffle { enabled: false }));
        assert!(!state.config.shuffle);
    }

    // ── PlaybackEvent::RepeatChanged ──────────────────────────────────────────

    #[test]
    fn repeat_changed_updates_config_repeat() {
        let mut state = AppState::default();
        assert_eq!(state.config.repeat, RepeatMode::Off);

        apply(&mut state, AppEvent::Playback(PlaybackEvent::RepeatChanged { mode: RepeatMode::All }));
        assert_eq!(state.config.repeat, RepeatMode::All);

        apply(&mut state, AppEvent::Playback(PlaybackEvent::RepeatChanged { mode: RepeatMode::One }));
        assert_eq!(state.config.repeat, RepeatMode::One);
    }

    // ── PlaybackEvent::Error ──────────────────────────────────────────────────

    #[test]
    fn error_event_sets_error_message() {
        let mut state = AppState::default();

        apply(&mut state, AppEvent::Playback(PlaybackEvent::Error {
            message: "codec error".to_owned(),
        }));

        assert_eq!(state.ui.error_message.as_deref(), Some("codec error"));
    }

    // ── LibraryEvent::ScanStarted ─────────────────────────────────────────────

    #[test]
    fn scan_started_sets_scanning_flag_and_status() {
        let mut state = AppState::default();
        let path = PathBuf::from("/music");

        apply(&mut state, AppEvent::Library(LibraryEvent::ScanStarted { path: path.clone() }));

        assert!(state.library.is_scanning);
        assert_eq!(state.library.last_scan_path, Some(path));
        assert!(state.ui.status_message.contains("Scanning"));
        assert!(state.ui.error_message.is_none());
    }

    // ── LibraryEvent::ScanProgress ────────────────────────────────────────────

    #[test]
    fn scan_progress_updates_status_with_count() {
        let mut state = AppState::default();

        apply(&mut state, AppEvent::Library(LibraryEvent::ScanProgress { found: 42 }));

        assert!(state.ui.status_message.contains("42"));
    }

    // ── LibraryEvent::ScanCompleted ───────────────────────────────────────────

    #[test]
    fn scan_completed_replaces_songs_and_clears_scanning_flag() {
        let mut state = state_with_songs(3);
        state.library.is_scanning = true;
        let new_songs: Vec<Song> = (0..7).map(|i| make_song(&format!("New {}", i))).collect();

        apply(&mut state, AppEvent::Library(LibraryEvent::ScanCompleted {
            songs: new_songs.clone(),
            count: 7,
        }));

        assert!(!state.library.is_scanning);
        assert_eq!(state.library.songs.len(), 7);
        assert!(state.ui.status_message.contains("7"));
    }

    #[test]
    fn scan_completed_auto_selects_first_when_nothing_selected() {
        let mut state = AppState::default();
        assert!(state.ui.selected_index.is_none());

        apply(&mut state, AppEvent::Library(LibraryEvent::ScanCompleted {
            songs: vec![make_song("First"), make_song("Second")],
            count: 2,
        }));

        assert_eq!(state.ui.selected_index, Some(0));
    }

    #[test]
    fn scan_completed_does_not_change_existing_selection() {
        let mut state = state_with_songs(5);
        state.ui.selected_index = Some(3);
        let new_songs: Vec<Song> = (0..5).map(|i| make_song(&format!("S{}", i))).collect();

        apply(&mut state, AppEvent::Library(LibraryEvent::ScanCompleted {
            songs: new_songs,
            count: 5,
        }));

        assert_eq!(state.ui.selected_index, Some(3), "existing selection must be preserved");
    }

    // ── LibraryEvent::LibraryLoaded ───────────────────────────────────────────

    #[test]
    fn library_loaded_replaces_songs() {
        let mut state = state_with_songs(2);
        let loaded: Vec<Song> = (0..4).map(|i| make_song(&format!("L{}", i))).collect();

        apply(&mut state, AppEvent::Library(LibraryEvent::LibraryLoaded { songs: loaded }));

        assert_eq!(state.library.songs.len(), 4);
    }

    #[test]
    fn library_loaded_auto_selects_first_when_nothing_selected() {
        let mut state = AppState::default();

        apply(&mut state, AppEvent::Library(LibraryEvent::LibraryLoaded {
            songs: vec![make_song("Only Song")],
        }));

        assert_eq!(state.ui.selected_index, Some(0));
    }

    #[test]
    fn library_loaded_preserves_existing_selection() {
        let mut state = state_with_songs(5);
        state.ui.selected_index = Some(2);

        apply(&mut state, AppEvent::Library(LibraryEvent::LibraryLoaded {
            songs: (0..5).map(|i| make_song(&format!("S{}", i))).collect(),
        }));

        assert_eq!(state.ui.selected_index, Some(2));
    }

    // ── LibraryEvent::SearchResults ───────────────────────────────────────────

    #[test]
    fn search_results_empty_sets_no_results_status() {
        let mut state = AppState::default();

        apply(&mut state, AppEvent::Library(LibraryEvent::SearchResults { results: vec![] }));

        assert!(state.ui.status_message.contains("No results"));
    }

    #[test]
    fn search_results_non_empty_auto_selects_first_and_updates_status() {
        let mut state = state_with_songs(5);
        let results = vec![(3, make_song("Match")), (1, make_song("Other"))];

        apply(&mut state, AppEvent::Library(LibraryEvent::SearchResults { results }));

        assert_eq!(state.ui.selected_index, Some(3), "first result's original index must be selected");
        assert!(state.ui.status_message.contains("2") || state.ui.status_message.contains("match"));
    }

    // ── UiEvent::SelectionChanged ─────────────────────────────────────────────

    #[test]
    fn selection_changed_updates_selected_index() {
        let mut state = AppState::default();

        apply(&mut state, AppEvent::Ui(UiEvent::SelectionChanged { index: 7 }));

        assert_eq!(state.ui.selected_index, Some(7));
    }

    // ── UiEvent::ShowMessage ──────────────────────────────────────────────────

    #[test]
    fn show_message_updates_status_and_clears_error() {
        let mut state = AppState::default();
        state.ui.error_message = Some("old error".to_owned());

        apply(&mut state, AppEvent::Ui(UiEvent::ShowMessage {
            message: "hello".to_owned(),
        }));

        assert_eq!(state.ui.status_message, "hello");
        assert!(state.ui.error_message.is_none());
    }

    // ── UiEvent::ShowError ────────────────────────────────────────────────────

    #[test]
    fn show_error_sets_error_message() {
        let mut state = AppState::default();

        apply(&mut state, AppEvent::Ui(UiEvent::ShowError {
            message: "bad path".to_owned(),
        }));

        assert_eq!(state.ui.error_message.as_deref(), Some("bad path"));
    }

    // ── UiEvent::SearchToggled ────────────────────────────────────────────────

    #[test]
    fn search_toggled_on_sets_search_active_and_status() {
        let mut state = AppState::default();

        apply(&mut state, AppEvent::Ui(UiEvent::SearchToggled { active: true }));

        assert!(state.ui.search_active);
        assert!(state.ui.status_message.contains("Search"));
    }

    #[test]
    fn search_toggled_off_clears_query_and_results() {
        let mut state = AppState::default();
        state.ui.search_active = true;
        state.ui.search_query = "pink".to_owned();
        state.ui.search_results = vec![(0, make_song("Pink"))];

        apply(&mut state, AppEvent::Ui(UiEvent::SearchToggled { active: false }));

        assert!(!state.ui.search_active);
        assert!(state.ui.search_query.is_empty());
        assert!(state.ui.search_results.is_empty());
    }

    #[test]
    fn search_toggled_off_with_playing_song_restores_selection_to_playing_index() {
        let mut state = state_with_songs(5);
        state.playback.current_index = Some(4);
        state.ui.search_active = true;
        state.ui.selected_index = Some(1); // search result was selected

        apply(&mut state, AppEvent::Ui(UiEvent::SearchToggled { active: false }));

        assert_eq!(
            state.ui.selected_index, Some(4),
            "selection must snap back to the currently playing song"
        );
    }

    #[test]
    fn search_toggled_off_without_playing_song_keeps_previous_selection() {
        let mut state = state_with_songs(5);
        state.playback.current_index = None;
        state.ui.selected_index = Some(2);
        state.ui.search_active = true;

        apply(&mut state, AppEvent::Ui(UiEvent::SearchToggled { active: false }));

        assert_eq!(state.ui.selected_index, Some(2));
    }

    #[test]
    fn search_toggled_off_with_no_selection_and_no_playing_selects_first() {
        let mut state = state_with_songs(3);
        state.playback.current_index = None;
        state.ui.selected_index = None;
        state.ui.search_active = true;

        apply(&mut state, AppEvent::Ui(UiEvent::SearchToggled { active: false }));

        assert_eq!(state.ui.selected_index, Some(0));
    }

    // ── UiEvent::SearchQueryChanged ───────────────────────────────────────────

    #[test]
    fn search_query_changed_updates_query_string() {
        let mut state = AppState::default();

        apply(&mut state, AppEvent::Ui(UiEvent::SearchQueryChanged {
            query: "bowie".to_owned(),
        }));

        assert_eq!(state.ui.search_query, "bowie");
    }

    // ── Shutdown / no-op ──────────────────────────────────────────────────────

    #[test]
    fn shutdown_event_does_not_mutate_state() {
        let state_before = AppState::default();
        let mut state = AppState::default();

        apply(&mut state, AppEvent::Shutdown);

        // Spot-check a few fields to confirm nothing changed.
        assert_eq!(state.config.shuffle, state_before.config.shuffle);
        assert_eq!(state.playback.is_playing, state_before.playback.is_playing);
    }
}