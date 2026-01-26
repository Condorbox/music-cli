use anyhow::Result;
use std::time::Duration;

use crate::player::audio::TuiPlayer;
use crate::ui::tui::{TuiUi, TuiEvent};

/// Orchestrates the TUI player - handles the event loop and coordinates
/// between the UI and audio player
pub fn run_tui_player(ui: &mut TuiUi) -> Result<()> {
    let mut player = TuiPlayer::new()?;

    loop {
        ui.render()?;

        // Check if song finished playing and auto-advance
        if !player.is_playing() && player.current_song().is_some() {
            ui.next_song();
            if let Some(song) = ui.get_selected_song() {
                let song = song.clone();
                player.play_song(&song)?;
                ui.set_playback_state(Some(&song), false);
            } else {
                ui.set_playback_state(None, false);
            }
        }

        // Handle user input
        if let Some(event) = ui.handle_input(Duration::from_millis(100))? {
            match event {
                TuiEvent::Quit => break,

                TuiEvent::PlaySelected => {
                    if let Some(song) = ui.get_selected_song() {
                        let song = song.clone();
                        player.play_song(&song)?;
                        ui.set_playback_state(Some(&song), false);
                    }
                }

                TuiEvent::TogglePause => {
                    player.toggle_pause();
                    if let Some(song) = player.current_song() {
                        ui.set_playback_state(Some(song), player.is_paused());
                    }
                }

                TuiEvent::NextTrack => {
                    ui.next_song();
                    if let Some(song) = ui.get_selected_song() {
                        let song = song.clone();
                        player.play_song(&song)?;
                        ui.set_playback_state(Some(&song), false);
                    }
                }

                TuiEvent::PreviousTrack => {
                    ui.previous_song();
                    if let Some(song) = ui.get_selected_song() {
                        let song = song.clone();
                        player.play_song(&song)?;
                        ui.set_playback_state(Some(&song), false);
                    }
                }

                TuiEvent::Navigate => {}
                TuiEvent::None => {}
            }
        }
    }

    Ok(())
}