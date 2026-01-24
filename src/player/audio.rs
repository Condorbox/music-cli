
use std::fs::File;
use std::io::{stdout, Write, BufReader};
use std::time::Duration;
use rodio::{Decoder, OutputStream, Sink};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{self, ClearType},
    ExecutableCommand,
    cursor
};
use anyhow::{Result, Context};

use crate::models::Song;

enum PlayerAction {
    Finished,
    Quit,
}



pub fn play_file(path: std::path::PathBuf) -> Result<()> {
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

    play_song(&song)
}

fn play_song(song: &Song) -> Result<()> {
    println!("Now playing: {}", song.title);

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let file = File::open(&song.path)?;

    let source = Decoder::new(BufReader::new(file))
        .with_context(|| format!("Failed to decode audio file: {}", song.path.display()))?;

    sink.append(source);

    terminal::enable_raw_mode()?;

    let action = player_loop(&sink, &song.title)?;

    terminal::disable_raw_mode()?;

    match action {
        PlayerAction::Finished => println!("\n✓ Playback ended"),
        PlayerAction::Quit => println!("\n✓ Playback stopped"),
    }

    Ok(())
}

pub fn play_playlist(songs: Vec<Song>) -> Result<()> {
    if songs.is_empty() {
        println!("No songs found to play.");
        return Ok(());
    }

    println!("Queueing {} songs...\n", songs.len());

    terminal::enable_raw_mode()?;

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let total_songs = songs.len();

    for (index, song) in songs.iter().enumerate() {
        println!("\n[{}/{}] Now playing: {}", index + 1, total_songs, song.title);

        let file = File::open(&song.path)?;
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Failed to decode audio file: {}", song.path.display()))?;

        sink.append(source);

        match player_loop(&sink, &song.title)? {
            PlayerAction::Finished => continue,
            PlayerAction::Quit => {
                terminal::disable_raw_mode()?;
                println!("\n✓ Playback stopped");
                return Ok(());
            }
        }
    }

    terminal::disable_raw_mode()?;
    println!("\n✓ Playlist finished");

    Ok(())
}

// TODO Refactor it
fn print_status(is_paused: bool, current_info: &str) {
    let mut stdout = stdout();

    // Move cursor to beginning of line and clear it
    stdout.execute(cursor::MoveToColumn(0)).ok();
    stdout.execute(terminal::Clear(ClearType::CurrentLine)).ok();

    print!("{} | {} | [Space/P/K: Pause/Play | Q/Esc: Quit]",
           if is_paused { "⏸ Paused " } else { "▶ Playing" },
           current_info);

    stdout.flush().ok();
}

fn clear_status_line() {
    let mut stdout = stdout();
    stdout.execute(cursor::MoveToColumn(0)).ok();
    stdout.execute(terminal::Clear(ClearType::CurrentLine)).ok();
    stdout.flush().ok();
}

// TODO Change the key handling
fn player_loop(sink: &Sink, title: &str) -> Result<PlayerAction> {
    let mut is_paused = false;
    print_status(is_paused, title);

    loop {
        if sink.empty() {
            return Ok(PlayerAction::Finished);
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char(' ') | KeyCode::Char('p') | KeyCode::Char('P') |
                    KeyCode::Char('k') | KeyCode::Char('K') => {
                        if is_paused {
                            sink.play();
                            is_paused = false;
                        } else {
                            sink.pause();
                            is_paused = true;
                        }
                        print_status(is_paused, title);
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        clear_status_line();
                        println!("\nQuitting...");
                        return Ok(PlayerAction::Quit);
                    }
                    _ => {}
                }
            }
        }
    }
}