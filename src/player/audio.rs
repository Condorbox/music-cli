
use std::fs::File;
use std::io::{BufReader};
use std::time::Duration;
use rodio::{Decoder, OutputStream, Sink};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{self},
};
use anyhow::{Result, Context};

use crate::models::Song;

use crate::ui::Ui;

enum PlayerAction {
    Finished,
    Quit,
    Next,
    Previous
}



pub fn play_file(path: std::path::PathBuf, ui: &mut impl Ui) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("File not found: {}", path.display());
    }

    if !path.is_file() {
        anyhow::bail!("Path is not a file: {}", path.display());
    }

    let song = Song {
        path: path.clone(),
        title: path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string(),
    };

    play_song(&song, ui)
}

fn play_song(song: &Song, ui: &mut impl Ui) -> Result<()> {
    ui.print_message(&format!("Now playing: {}", song.title));

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let file = File::open(&song.path)?;

    let source = Decoder::new(BufReader::new(file))
        .with_context(|| format!("Failed to decode audio file: {}", song.path.display()))?;

    sink.append(source);

    terminal::enable_raw_mode()?;

    let action = player_loop(&sink, &song.title, ui)?;

    terminal::disable_raw_mode()?;

    match action {
        PlayerAction::Finished | PlayerAction::Next | PlayerAction::Previous => ui.print_message("\n✓ Playback ended"),
        PlayerAction::Quit  => ui.print_message("\n✓ Playback stopped"),
    }

    Ok(())
}

pub fn play_playlist(songs: Vec<Song>, ui: &mut impl Ui) -> Result<()> {
    if songs.is_empty() {
        ui.print_message("No songs found to play.");
        return Ok(());
    }

    ui.print_message(&format!("Queueing {} songs...\n", songs.len()));

    terminal::enable_raw_mode()?;

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let total_songs = songs.len();
    let mut current_index = 0;

    while current_index < total_songs {
        let song = &songs[current_index];

        ui.print_message(&format!("\n[{}/{}] Now playing: {}", current_index + 1, total_songs, song.title));

        let file = File::open(&song.path)?;
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Failed to decode audio file: {}", song.path.display()))?;

        sink.append(source);

        match player_loop(&sink, &song.title, ui)? {
            PlayerAction::Finished | PlayerAction::Next => {
                current_index += 1;
            }
            PlayerAction::Previous => {
                if current_index > 0 {
                    current_index -= 1;
                }
            }
            PlayerAction::Quit => {
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

// TODO Change the key handling
fn player_loop(sink: &Sink, title: &str, ui: &mut impl Ui) -> Result<PlayerAction> {
    let mut is_paused = false;
    ui.show_status(is_paused, title);

    loop {
        if sink.empty() {
            return Ok(PlayerAction::Finished);
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    // Pause/Resume
                    KeyCode::Char(' ') | KeyCode::Char('p') | KeyCode::Char('P') |
                    KeyCode::Char('k') | KeyCode::Char('K') => {
                        if is_paused {
                            sink.play();
                            is_paused = false;
                        } else {
                            sink.pause();
                            is_paused = true;
                        }
                        ui.show_status(is_paused, title);
                    }
                    // Next Song
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Right => {
                        sink.stop();
                        return Ok(PlayerAction::Next);
                    }
                    // Previous Song
                    KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Left => {
                        sink.stop();
                        return Ok(PlayerAction::Previous);
                    }
                    // Quit
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        ui.print_message("\nQuitting...");
                        return Ok(PlayerAction::Quit);
                    }
                    _ => {}
                }
            }
        }
    }
}