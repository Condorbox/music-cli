use std::time::Duration;
use crate::application::state::{AppState, UiState};
use crate::core::events::UiEvent;
use crate::core::models::Song;
use anyhow::Result;

// TODO Implement volume control

/// Abstraction for audio playback backend
pub trait PlaybackBackend: Send {
    /// Play a song (replaces current playback)
    fn play(&mut self, song: &Song) -> Result<()>;

    /// Stop playback
    fn stop(&mut self);

    /// Pause playback
    fn pause(&mut self);

    /// Resume playback
    fn resume(&mut self);

    /// Check if currently playing
    fn is_playing(&self) -> bool;

    /// Check if paused
    fn is_paused(&self) -> bool;

    /// Check if track has finished
    fn has_finished(&self) -> bool;

    /// Get current song
    fn current_song(&self) -> Option<&Song>;

    /// Set volume (0.0 - 1.0)
    fn set_volume(&mut self, volume: f32);

    /// Get volume
    fn volume(&self) -> f32;

    /// Get current playback position (elapsed time)
    /// Returns Duration::ZERO if not playing
    fn position(&self) -> Duration {
        Duration::ZERO
    }
}

/// Abstraction for persistent storage
pub trait StorageBackend: Send {
    /// Load application state
    fn load(&self) -> Result<AppState>;

    /// Save application state
    fn save(&self, state: &AppState) -> Result<()>;
}

/// Abstraction for UI rendering
pub trait UiRenderer: Send {
    /// Initialize the UI (setup terminal, etc.)
    fn init(&mut self) -> Result<()>;

    /// Cleanup the UI (restore terminal, etc.)
    fn cleanup(&mut self) -> Result<()>;

    /// Render current state
    fn render(&mut self, state: &UiState) -> Result<()>;

    /// Poll for user input (non-blocking)
    /// Returns events generated from user input
    fn poll_input(&mut self) -> Result<Vec<UiEvent>>;

    /// Update renderer with current app state before rendering
    /// Default implementation does nothing
    fn update_state(&mut self, _state: &AppState) {
        
    }
    
    /// Get as Any for downcasting (needed for renderer-specific updates)
    fn as_any(&mut self) -> &mut dyn std::any::Any;
}
