use crate::application::handlers::HandlerContext;
use crate::core::events::{AppEvent, PlaybackEvent};
use anyhow::Result;
use crate::core::models::RepeatMode;

/// Handles all [`PlaybackEvent`] variants.
///
/// Responsible for:
/// - Driving the playback backend (play, pause, resume, volume)
/// - Auto-advancing to the next track when one finishes
/// - Persisting volume and shuffle changes to storage
pub struct PlaybackHandler;

impl PlaybackHandler {
    pub fn handle(&self, event: &PlaybackEvent, ctx: &mut HandlerContext) -> Result<()> {
        match event {
            PlaybackEvent::PlayRequested { song } => {
                if let Some(playback) = ctx.playback.as_mut() {
                    playback.play(song)?;
                    ctx.event_tx
                        .send(AppEvent::Playback(PlaybackEvent::Started {
                            song: song.clone(),
                        }))?;
                }
            }

            PlaybackEvent::TrackFinished => {
                // Read everything we need from state while holding the lock, then drop it.
                let (repeat, current_index, library_len) = {
                    let state = ctx.state.lock().unwrap();
                    (
                        state.config.repeat,
                        state.playback.current_index, // authoritative index of what was playing
                        state.library.songs.len(),
                    )
                };

                match repeat {
                    // Repeat the same song — ignore shuffle and loop settings.
                    RepeatMode::One => {
                        if let Some(idx) = current_index {
                            let song = ctx.state.lock().unwrap().library.songs.get(idx).cloned();
                            if let Some(song) = song {
                                ctx.event_tx
                                    .send(AppEvent::Playback(PlaybackEvent::PlayRequested { song }))?;
                            }
                        }
                    }

                    // Loop playlist when exhausted.
                    RepeatMode::All => {
                        ctx.advance_to_next(current_index, library_len, true)?;
                    }

                    // Stop at the end of the playlist.
                    RepeatMode::Off => {
                        ctx.advance_to_next(current_index, library_len, false)?;
                    }
                }
            }

            PlaybackEvent::VolumeChanged { volume } => {
                if let Some(playback) = ctx.playback.as_mut() {
                    playback.set_volume(*volume);
                }
                ctx.persist_state()?;
            }

            PlaybackEvent::Shuffle { .. } => {
                // State already updated by AppState::apply_event before this handler runs.
                ctx.persist_state()?;
            }

            PlaybackEvent::RepeatChanged { .. } => {
                // State already updated by AppState::apply_event before this handler runs.
                ctx.persist_state()?;
            }

            // All other variants (Started, Paused, Resumed, Stopped, Error) only
            // update state — already handled by AppState::apply_event.
            PlaybackEvent::Started { .. }
            | PlaybackEvent::Paused
            | PlaybackEvent::Resumed
            | PlaybackEvent::Stopped
            | PlaybackEvent::Error { .. } => {}
        }

        Ok(())
    }
}