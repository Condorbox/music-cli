
use std::fs::File;
use std::io::{stdout, Write, BufReader};
use std::time::Duration;
use rodio::{Decoder, OutputStream, Sink};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{self, ClearType},
    ExecutableCommand,
};

pub fn play_file(path: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()).into());
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()).into());
    }

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let file = File::open(&path)?;
    let source = Decoder::new(BufReader::new(file)).map_err(|e| {
        format!("Error decoding audio file {}: {}", path.display(), e)
    })?;

    sink.append(source);

    // Enable raw mode for keyboard input
    terminal::enable_raw_mode()?;

    let result = player_loop(&sink);

    // Cleanup: disable raw mode
    terminal::disable_raw_mode()?;
    println!("\nPlayback ended");

    result
}

fn print_status(is_paused: bool) {
    let mut stdout = stdout();
    stdout.execute(terminal::Clear(ClearType::CurrentLine)).ok();
    print!("\r{} ", if is_paused { "⏸ Paused " } else { "▶ Playing" });
    stdout.flush().ok();
}

// TODO Change the key handling
fn player_loop(sink: &Sink) -> Result<(), Box<dyn std::error::Error>> {
    let mut is_paused = false;
    print_status(is_paused);

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
                        print_status(is_paused);
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