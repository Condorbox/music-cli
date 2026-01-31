use crate::core::models::Song;
use std::path::PathBuf;

/// All events that can occur in the application
#[derive(Debug, Clone)]
pub enum AppEvent {
    // Playback events
    Playback(PlaybackEvent),

    // Library events
    Library(LibraryEvent),

    // UI events
    Ui(UiEvent),

    // Application lifecycle
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    /// Request to play a specific song
    PlayRequested { song: Song },

    /// Playback started
    Started { song: Song },

    /// Playback paused
    Paused,

    /// Playback resumed
    Resumed,

    /// Current track finished
    TrackFinished,

    /// Playback stopped
    Stopped,

    /// Playback error occurred
    Error { message: String },

    /// Volume changed (0.0 - 1.0)
    VolumeChanged { volume: f32 },
}

#[derive(Debug, Clone)]
pub enum LibraryEvent {
    /// Request to scan directory
    ScanRequested { path: PathBuf },

    /// Scanning started
    ScanStarted { path: PathBuf },

    /// Scan progress update
    ScanProgress { found: usize },

    /// Scanning completed
    ScanCompleted { songs: Vec<Song>, count: usize },

    /// Library loaded from storage
    LibraryLoaded { songs: Vec<Song> },

    /// Search requested
    SearchRequested { query: String },

    /// Search results
    SearchResults { results: Vec<(usize, Song)> },
}

#[derive(Debug, Clone)]
pub enum UiEvent {
    /// User requested to play selected song
    PlaySelectedRequested,

    /// User requested pause/resume toggle
    TogglePauseRequested,

    /// User requested next track
    NextTrackRequested,

    /// User requested previous track
    PreviousTrackRequested,

    /// User changed selection
    SelectionChanged { index: usize },

    /// User requested quit
    QuitRequested,

    /// Display message to user
    ShowMessage { message: String },

    /// Display error to user
    ShowError { message: String },

    /// User requested volume change (0-100)
    VolumeChangeRequested { volume: u8 },

    /// User requested path change
    PathChangeRequested { path: PathBuf },
}

/// Type alias for event sender
pub type EventSender = crossbeam_channel::Sender<AppEvent>;

/// Type alias for event receiver
pub type EventReceiver = crossbeam_channel::Receiver<AppEvent>;