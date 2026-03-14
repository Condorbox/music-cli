use crate::application::state::{AppState, UiState};
use crate::core::events::UiEvent;
use crate::core::models::Song;
use crate::core::traits::UiRenderer;
use crate::modules::playback::playback_progress::PlaybackProgress;
use crate::modules::ui::progress_formatter::format_duration;
use crate::modules::ui::input::{map_key, InputAction, InputMode};
use crate::utils::PROGRESS_BAR_WIDTH;
use anyhow::Result;
use crossterm::cursor::MoveTo;
use crossterm::{event::{self, Event}, queue, terminal::{self, Clear, ClearType}};
use std::io::{stdout, Write};
use std::time::Duration;

pub struct TerminalRenderer {
    initialized: bool,
    shuffle_enabled: bool,
    current_song: Option<Song>,
    current_elapsed: Duration,
    is_paused: bool,
}

impl TerminalRenderer {
    pub fn new() -> Self {
        Self {
            initialized: false,
            shuffle_enabled: false,
            current_song: None,
            current_elapsed: Duration::from_secs(0),
            is_paused: false,
        }
    }

    pub fn print_message(&self, message: &str) {
        println!("{}", message);
    }

    pub fn print_error(&self, message: &str) {
        eprintln!("Error: {}", message);
    }

    pub fn print_song_list(&self, songs: &[Song]) {
        let total = songs.len();
        for (index, song) in songs.iter().enumerate() {
            println!("[{}/{}] {}", index + 1, total, song);
        }
    }

    pub fn print_song_list_refs(&self, songs: &[&Song]) {
        let total = songs.len();
        for (index, song) in songs.iter().enumerate() {
            println!("[{}/{}] {}", index + 1, total, song);
        }
    }

    pub fn print_search_results(&self, query: &str, results: &[(usize, Song)]) {
        if results.is_empty() {
            println!("No songs found matching: '{}'", query);
        } else {
            println!("Found {} matches:", results.len());
            for (index, song) in results {
                println!("[{}] {}", index, song);
            }
        }
    }

    fn render_progress_bar(&self, stdout: &mut impl Write) -> Result<()> {
        if let Some(song) = &self.current_song {
            if let Some(total) = song.duration {
                if let Some(progress) = PlaybackProgress::new(self.current_elapsed, total) {
                    let filled =
                        (progress.ratio() * PROGRESS_BAR_WIDTH as f64).round() as usize;

                    let filled = filled.min(PROGRESS_BAR_WIDTH);
                    let empty = PROGRESS_BAR_WIDTH - filled;

                    write!(
                        stdout,
                        "  {} [{}{}] {}",
                        format_duration(progress.elapsed()),
                        "█".repeat(filled),
                        "░".repeat(empty),
                        format_duration(progress.total()),
                    )?;

                    return Ok(());
                }
            }
        }

        // Fallback placeholder
        write!(
            stdout,
            "  --:-- [{}] --:--",
            "░".repeat(PROGRESS_BAR_WIDTH)
        )?;

        Ok(())
    }
}

impl UiRenderer for TerminalRenderer {
    fn init(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        self.initialized = true;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        if self.initialized {
            // execute!(stdout(), LeaveAlternateScreen)?;
            terminal::disable_raw_mode()?;
            self.initialized = false;
        }
        Ok(())
    }

    fn render(&mut self, state: &UiState) -> Result<()> {
        let mut stdout = stdout();

        // Clear screen from top
        queue!(
            stdout,
            MoveTo(0, 0),
            Clear(ClearType::FromCursorDown)
        )?;

        // Status
        let status = if self.is_paused { "⏸ PAUSED" } else { "▶ PLAYING" };
        let shuffle = if self.shuffle_enabled { "🔀  Shuffle" } else { "▶️  Linear" };

        let error_part = state
            .error_message
            .as_ref()
            .map(|e| format!(" | ERROR: {}", e))
            .unwrap_or_default();

        queue!(stdout, MoveTo(0, 0))?;
        write!(
            stdout,
            "{} | {} | {}{}",
            status,
            shuffle,
            state.status_message,
            error_part
        )?;

        // Song Info
        queue!(stdout, MoveTo(0, 1))?;

        if let Some(song) = &self.current_song {
            let artist = song.format_artists();
            let album = song.album.as_deref().unwrap_or("Unknown Album");

            write!(stdout, "  {} — {} • {}", song.title, artist, album)?;
        } else {
            write!(stdout, "  No song playing")?;
        }

        // Progress Bar
        queue!(stdout, MoveTo(0, 2))?;
        self.render_progress_bar(&mut stdout)?;

        // Controls
        queue!(stdout, MoveTo(0, 3))?;
        write!(
            stdout,
            "  [Space: Pause | N: Next | B: Prev | R: Shuffle | Q: Quit]"
        )?;

        stdout.flush()?;
        Ok(())
    }

    fn poll_input(&mut self) -> Result<Vec<UiEvent>> {
        let mut events = Vec::new();

        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if let Some(action) = map_key(InputMode::Normal, key) {
                    self.apply_action(action, &mut events);
                }
            }
        }

        Ok(events)
    }

    fn update_state(&mut self, state: &AppState) {
        self.shuffle_enabled = state.config.shuffle;
        self.current_song = state.playback.current_song.clone();
        self.current_elapsed = state.playback.current_elapsed;
        self.is_paused = state.playback.is_paused;
    }
}

impl TerminalRenderer {
    fn apply_action(&self, action: InputAction, events: &mut Vec<UiEvent>) {
        match action {
            InputAction::TogglePause => events.push(UiEvent::TogglePauseRequested),
            InputAction::NextTrack => events.push(UiEvent::NextTrackRequested),
            InputAction::PreviousTrack => events.push(UiEvent::PreviousTrackRequested),
            InputAction::ToggleShuffle => events.push(UiEvent::ShuffleToggled {
                shuffle_enabled: self.shuffle_enabled,
            }),
            InputAction::Quit => events.push(UiEvent::QuitRequested),
            InputAction::PlaySelected => events.push(UiEvent::PlaySelectedRequested),
            InputAction::Refresh => events.push(UiEvent::RefreshRequested),
            _ => {}
        }
    }
}
