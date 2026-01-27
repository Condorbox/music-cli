use anyhow::Result;
use std::time::Duration;

use crate::player::audio_player::AudioPlayer;
use crate::ui::tui::{TuiUi, TuiEvent};

/// TUI-specific controller that drives the AudioPlayer
pub struct TuiController;

impl TuiController {
    /// Run the TUI event loop with the given player
    pub fn run(ui: &mut TuiUi, player: &mut AudioPlayer) -> Result<()> {
        loop {
            ui.render()?;

            // Auto-advance to next song when current finishes
            if player.has_finished() {
                ui.next_song();
                if let Some(song) = ui.get_selected_song() {
                    player.play(&song)?;
                    ui.set_playback_state(Some(&song), false);
                }
            }

            // Handle user input
            if let Some(event) = ui.handle_input(Duration::from_millis(100))? {
                match event {
                    TuiEvent::Quit => break,

                    TuiEvent::PlaySelected => {
                        if let Some(song) = ui.get_selected_song() {
                            player.play(&song)?;
                            ui.set_playback_state(Some(&song), false);
                        } else {
                            ui.set_playback_state(None, false);
                        }
                    }

                    TuiEvent::NextTrack => {
                        ui.next_song();
                        if let Some(song) = ui.get_selected_song() {
                            player.play(&song)?;
                            ui.set_playback_state(Some(&song), false);
                        }
                    }

                    TuiEvent::PreviousTrack => {
                        ui.previous_song();
                        if let Some(song) = ui.get_selected_song() {
                            player.play(&song)?;
                            ui.set_playback_state(Some(&song), false);
                        }
                    }

                    TuiEvent::TogglePause => {
                        player.toggle_pause();
                        ui.set_playback_state(player.current_song(), player.is_paused());
                    }

                    TuiEvent::Navigate | TuiEvent::None => {}
                }
            }
        }

        Ok(())
    }
}