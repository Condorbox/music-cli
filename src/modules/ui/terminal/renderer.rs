use crate::application::state::UiState;
use crate::core::events::UiEvent;
use crate::core::models::Song;
use crate::core::traits::UiRenderer;
use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{self, ClearType},
    ExecutableCommand,
};
use std::io::{stdout, Write};
use std::time::Duration;

pub struct TerminalRenderer {
    initialized: bool,
}

impl TerminalRenderer {
    pub fn new() -> Self {
        Self { initialized: false }
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
}

impl UiRenderer for TerminalRenderer {
    fn init(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        self.initialized = true;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        if self.initialized {
            terminal::disable_raw_mode()?;
            self.initialized = false;
        }
        Ok(())
    }

    fn render(&mut self, state: &UiState) -> Result<()> {
        let mut stdout = stdout();

        stdout.execute(cursor::MoveToColumn(0))?;
        stdout.execute(terminal::Clear(ClearType::CurrentLine))?;

        // Show status message
        print!("{}", state.status_message);

        // Show error if any
        if let Some(error) = &state.error_message {
            print!(" | ERROR: {}", error);
        }

        print!(" | [Space: Pause | N: Next | B: Prev | Q: Quit]");

        stdout.flush()?;
        Ok(())
    }

    fn poll_input(&mut self) -> Result<Vec<UiEvent>> {
        let mut events = Vec::new();

        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char(' ') | KeyCode::Char('p') | KeyCode::Char('P') |
                    KeyCode::Char('k') | KeyCode::Char('K') => {
                        events.push(UiEvent::TogglePauseRequested);
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Right => {
                        events.push(UiEvent::NextTrackRequested);
                    }
                    KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Left => {
                        events.push(UiEvent::PreviousTrackRequested);
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        events.push(UiEvent::QuitRequested);
                    }
                    KeyCode::Enter => {
                        events.push(UiEvent::PlaySelectedRequested);
                    }
                    _ => {}
                }
            }
        }

        Ok(events)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
