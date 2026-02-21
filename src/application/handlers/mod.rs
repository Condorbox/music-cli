pub mod library_handler;
pub mod playback_handler;
pub mod ui_handler;

use crate::application::state::AppState;
use crate::core::events::{AppEvent, EventSender, PlaybackEvent};
use crate::core::models::Song;
use crate::core::traits::{PlaybackBackend, StorageBackend};
use crate::modules::playback::shuffle_manager::ShuffleManager;
use anyhow::Result;
use std::sync::{Arc, Mutex};


/// Describes what `execute_nav` should do after navigation logic resolves.
pub enum NavTarget {
    /// Move to a different song at this index.
    Go(usize),
    /// Replay the currently-playing song from the beginning.
    Restart,
    /// No playback action (nothing is selected/playing yet).
    Nothing,
}

/// All dependencies that event handlers need to do their work.
///
/// Passed by `&mut` into each handler method, giving handlers access to
/// shared resources without coupling them to the `Application` struct itself.
pub struct HandlerContext<'a> {
    pub state: &'a Arc<Mutex<AppState>>,
    pub event_tx: &'a EventSender,
    pub playback: &'a mut Option<Box<dyn PlaybackBackend>>,
    pub storage: &'a Option<Box<dyn StorageBackend>>,
    pub shuffle_manager: &'a mut ShuffleManager,
}

impl<'a> HandlerContext<'a> {
    /// Save the current state to storage, if a backend is present.
    pub fn persist_state(&self) -> Result<()> {
        if let Some(storage) = self.storage {
            let state = self.state.lock().unwrap();
            storage.save(&state)?;
        }

        Ok(())
    }

    /// Advance to the next track, respecting shuffle mode and the `loop_playlist` flag.
    ///
    /// - Shuffle on: delegates to `ShuffleManager::next_index`. When the queue is exhausted
    ///   and `loop_playlist` is false, falls back to `NavTarget::Restart` (replay current).
    /// - Shuffle off, sequential: `idx+1` if in range; wraps to 0 when `loop_playlist` is
    ///   true; falls back to `NavTarget::Restart` at end when looping is off.
    pub fn advance_to_next(
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
                None => NavTarget::Restart,
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
                        NavTarget::Restart
                    }
                }
                None => NavTarget::Nothing,
            }
        };

        self.execute_nav(target, current_index)
    }

    /// Go back to the previous track, respecting shuffle mode and the `loop_playlist` flag.
    ///
    /// - Shuffle on: walks back through the existing shuffle history via
    ///   `ShuffleManager::previous_index`. Falls back to `NavTarget::Restart` at the start.
    /// - Shuffle off, sequential: at index 0 wraps to the last song when `loop_playlist`
    ///   is true, otherwise restarts the current song.
    pub fn advance_to_prev(
        &mut self,
        current_index: Option<usize>,
        library_len: usize,
        loop_playlist: bool,
    ) -> Result<()> {
        let target = if self.shuffle_manager.is_enabled() {
            match self.shuffle_manager.previous_index(current_index) {
                Some(idx) => NavTarget::Go(idx),
                None => NavTarget::Restart,
            }
        } else {
            match current_index {
                Some(0) => {
                    if loop_playlist {
                        NavTarget::Go(library_len.saturating_sub(1))
                    } else {
                        NavTarget::Restart
                    }
                }
                Some(idx) => NavTarget::Go(idx - 1),
                None => NavTarget::Nothing,
            }
        };

        self.execute_nav(target, current_index)
    }

    /// Resolves a `NavTarget` into a `PlayRequested` event (or nothing).
    ///
    /// - `Go(idx)` → update `selected_index` to `idx` and play that song.
    /// - `Restart`  → replay `current_index` (the song that was already playing).
    /// - `Nothing`  → no-op.
    fn execute_nav(&self, target: NavTarget, current_index: Option<usize>) -> Result<()> {
        let play_index = match target {
            NavTarget::Go(idx) => Some(idx),
            NavTarget::Restart => current_index,
            NavTarget::Nothing => None,
        };

        if let Some(idx) = play_index {
            let song: Option<Song> = {
                let mut state = self.state.lock().unwrap();
                state.ui.selected_index = Some(idx);
                state.library.songs.get(idx).cloned()
            };

            if let Some(song) = song {
                self.event_tx
                    .send(AppEvent::Playback(PlaybackEvent::PlayRequested { song }))?;
            }
        }

        Ok(())
    }
}

