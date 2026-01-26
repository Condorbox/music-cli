
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
    
    play_song(&Song::from_path(&path), ui)
}

pub fn play_song(song: &Song, ui: &mut impl Ui) -> Result<()> {
    ui.print_message(&format!("Now playing: {}", song.title));

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let file = File::open(&song.path)?;

    let source = Decoder::new(BufReader::new(file))
        .with_context(|| format!("Failed to decode audio file: {}", song.path.display()))?;

    sink.append(source);

    terminal::enable_raw_mode()?;

    let action = player_loop(&sink, &song, ui)?;

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

        match player_loop(&sink, &song, ui)? {
            PlayerAction::Finished | PlayerAction::Next => {
                current_index += 1;
            }
            PlayerAction::Previous => {
                // TODO Maybe if the song is less than x% go to the start of the song b
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
fn player_loop(sink: &Sink, song:  &Song, ui: &mut impl Ui) -> Result<PlayerAction> {
    let mut is_paused = false;
    ui.show_status(is_paused, song);

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
                        ui.show_status(is_paused, song);
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

// TODO Make a Player interface so is the same code for terminal and TUI
pub struct TuiPlayer {
    _stream: OutputStream,
    sink: Sink,
    current_song: Option<Song>,
    is_paused: bool,
}

impl TuiPlayer {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;

        Ok(Self {
            _stream,
            sink,
            current_song: None,
            is_paused: false,
        })
    }

    pub fn play_song(&mut self, song: &Song) -> Result<()> {
        self.sink.stop();

        let file = File::open(&song.path)?;
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Failed to decode audio file: {}", song.path.display()))?;

        self.sink.append(source);
        self.current_song = Some(song.clone());
        self.is_paused = false;

        Ok(())
    }

    pub fn toggle_pause(&mut self) {
        if self.current_song.is_some() {
            if self.is_paused {
                self.sink.play();
                self.is_paused = false;
            } else {
                self.sink.pause();
                self.is_paused = true;
            }
        }
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn is_playing(&self) -> bool {
        self.current_song.is_some() && !self.sink.empty()
    }

    pub fn current_song(&self) -> Option<&Song> {
        self.current_song.as_ref()
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        self.current_song = None;
        self.is_paused = false;
    }
}
