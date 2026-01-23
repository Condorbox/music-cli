
use std::fs::File;
use std::io::{stdout, Write, BufReader};
use std::time::Duration;
use rodio::{Decoder, OutputStream, Sink};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{self, ClearType},
    ExecutableCommand,
};
use anyhow::{Result, Context};

use crate::models::Song;

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

    // Enable raw mode for keyboard input
    terminal::enable_raw_mode()?;

    let result = player_loop(&sink, &song.title);

    // Disable raw mode
    terminal::disable_raw_mode()?;
    println!("\n✓ Playback ended");

    result
}


fn print_status(is_paused: bool, current_info: &str) {
    let mut stdout = stdout();
    stdout.execute(terminal::Clear(ClearType::CurrentLine)).ok();
    print!("\r{} | {} | [Space/P/K: Pause/Play | Q/Esc: Quit]",
           if is_paused { "⏸ Paused " } else { "▶ Playing" },
           current_info);
    stdout.flush().ok();
}

// TODO Change the key handling
fn player_loop(sink: &Sink, title: &str) -> Result<()> {
    let mut is_paused = false;
    print_status(is_paused, title);

    loop {
        // Check if playback has ended
        if sink.empty() {
            break;
        }
        // Poll for keyboard events with timeout
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
                        println!("\nQuitting...");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}