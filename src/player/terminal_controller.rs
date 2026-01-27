use anyhow::Result;
use std::time::Duration;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal,
};

use crate::models::Song;
use crate::player::audio_player::AudioPlayer;
use crate::ui::Ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackResult {
    Finished,
    Quit,
    Next,
    Previous,
}

pub struct TerminalController;

impl TerminalController {
    /// Play a single song with keyboard controls
    pub fn play_song(player: &mut AudioPlayer, song: &Song, ui: &mut impl Ui) -> Result<PlaybackResult> {
        ui.print_message(&format!("Now playing: {}", song.title));

        player.play(song)?;

        terminal::enable_raw_mode()?;
        let result = Self::run_event_loop(player, song, ui)?;
        terminal::disable_raw_mode()?;

        match result {
            PlaybackResult::Finished | PlaybackResult::Next | PlaybackResult::Previous => {
                ui.print_message("\n✓ Playback ended")
            }
            PlaybackResult::Quit => ui.print_message("\n✓ Playback stopped"),
        }

        Ok(result)
    }

    /// Play a playlist of songs
    pub fn play_playlist(player: &mut AudioPlayer, songs: Vec<Song>, ui: &mut impl Ui) -> Result<()> {
        if songs.is_empty() {
            ui.print_message("No songs found to play.");
            return Ok(());
        }

        ui.print_message(&format!("Queueing {} songs...\n", songs.len()));

        terminal::enable_raw_mode()?;

        let total_songs = songs.len();
        let mut current_index = 0;

        while current_index < total_songs {
            let song = &songs[current_index];

            ui.print_message(&format!(
                "\n[{}/{}] Now playing: {}",
                current_index + 1,
                total_songs,
                song.title
            ));

            player.play(song)?;

            match Self::run_event_loop(player, song, ui)? {
                PlaybackResult::Finished | PlaybackResult::Next => {
                    current_index += 1;
                }
                PlaybackResult::Previous => {
                    if current_index > 0 {
                        current_index -= 1;
                    }
                }
                PlaybackResult::Quit => {
                    terminal::disable_raw_mode()?;
                    ui.print_message("\n✓ Playback stopped");
                    return Ok(());
                }
            }
        }

        terminal::disable_raw_mode()?;
        ui.print_message("\n✓ Playlist finished");

        Ok(())
    }

    fn run_event_loop(player: &mut AudioPlayer, song: &Song, ui: &mut impl Ui) -> Result<PlaybackResult> {
        ui.show_status(player.is_paused(), song);

        loop {
            if player.has_finished() {
                return Ok(PlaybackResult::Finished);
            }

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    match code {
                        // Pause/Resume
                        KeyCode::Char(' ')
                        | KeyCode::Char('p')
                        | KeyCode::Char('P')
                        | KeyCode::Char('k')
                        | KeyCode::Char('K') => {
                            player.toggle_pause();
                            ui.show_status(player.is_paused(), song);
                        }
                        // Next Song
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Right => {
                            player.stop();
                            return Ok(PlaybackResult::Next);
                        }
                        // Previous Song
                        KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Left => {
                            player.stop();
                            return Ok(PlaybackResult::Previous);
                        }
                        // Quit
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            ui.print_message("\nQuitting...");
                            return Ok(PlaybackResult::Quit);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}