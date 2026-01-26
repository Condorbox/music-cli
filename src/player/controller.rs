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

        if !player.is_playing() && player.current_song().is_some() {
            ui.next_song();
            play_active_selection(ui, &mut player)?;
        }

        if let Some(event) = ui.handle_input(Duration::from_millis(100))? {
            match event {
                TuiEvent::Quit => break,

                TuiEvent::PlaySelected => {
                    play_active_selection(ui, &mut player)?;
                }

                TuiEvent::NextTrack => {
                    ui.next_song();
                    play_active_selection(ui, &mut player)?;
                }

                TuiEvent::PreviousTrack => {
                    ui.previous_song();
                    play_active_selection(ui, &mut player)?;
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

fn play_active_selection(ui: &mut TuiUi, player: &mut TuiPlayer) -> Result<()> {
    if let Some(song) = ui.get_selected_song() {
        let song = song.clone();
        player.play_song(&song)?;
        ui.set_playback_state(Some(&song), false);
    } else {
        // Handle case where list might be empty
        ui.set_playback_state(None, false);
    }

    Ok(())
}