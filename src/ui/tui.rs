use std::io::{stdout, Stdout};
use std::cell::RefCell;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::time::Duration;

use crate::models::Song;
use crate::ui::Ui;
use crate::utils::APP_NAME;

pub struct TuiUi {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    list_state: RefCell<ListState>,
    songs: Vec<Song>,
    status_message: String,
    current_song: Option<Song>,
    is_paused: bool,
}

impl TuiUi {
    pub fn new() -> Self {
        Self {
            terminal: None,
            list_state: RefCell::new(ListState::default()),
            songs: Vec::new(),
            status_message: String::from("Welcome to Music CLI"),
            current_song: None,
            is_paused: false,
        }
    }

    pub fn init(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        self.terminal = Some(Terminal::new(backend)?);
        Ok(())
    }

    pub fn cleanup(&mut self) -> anyhow::Result<()> {
        disable_raw_mode()?;
        if let Some(mut terminal) = self.terminal.take() {
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
        }
        Ok(())
    }

    pub fn set_songs(&mut self, songs: Vec<Song>) {
        self.songs = songs;
        if !self.songs.is_empty() && self.list_state.borrow().selected().is_none() {
            self.list_state.borrow_mut().select(Some(0));
        }
    }

    pub fn get_selected_song(&self) -> Option<Song> {
        self.list_state.borrow().selected()
            .and_then(|i| self.songs.get(i).cloned())
    }

    pub fn get_selected_index(&self) -> Option<usize> {
        self.list_state.borrow().selected()
    }

    pub fn next_song(&mut self) {
        let mut state = self.list_state.borrow_mut();
        let i = match state.selected() {
            Some(i) => {
                if i >= self.songs.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    pub fn previous_song(&mut self) {
        let mut state = self.list_state.borrow_mut();
        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    self.songs.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        // Take the terminal out temporarily to avoid borrow conflicts
        let mut terminal = match self.terminal.take() {
            Some(t) => t,
            None => return Ok(()),
        };

        // Now self is not borrowed, so the closure can borrow it
        terminal.draw(|f| self.draw_ui(f))?;

        // Put the terminal back
        self.terminal = Some(terminal);
        Ok(())
    }

    pub fn set_playback_state(&mut self, song: Option<&Song>, is_paused: bool) {
        self.current_song = song.cloned();
        self.is_paused = is_paused;
    }

    pub fn handle_input(&mut self, timeout: Duration) -> anyhow::Result<Option<TuiEvent>> {
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                return Ok(Some(self.process_key(key)));
            }
        }
        Ok(None)
    }

    fn draw_ui(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Main content
                Constraint::Length(4),  // Now playing
                Constraint::Length(3),  // Controls
            ])
            .split(f.area());

        self.draw_header(f, chunks[0]);
        self.draw_song_list(f, chunks[1]);
        self.draw_now_playing(f, chunks[2]);
        self.draw_controls(f, chunks[3]);
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new(format!("♪ {} Player ♪ ", APP_NAME))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn draw_song_list(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .songs
            .iter()
            .enumerate()
            .map(|(i, song)| {
                let content = format!(
                    "{:3}. {} - {} [{}]",
                    i + 1,
                    song.artist.as_deref().unwrap_or("Unknown"),
                    song.title,
                    song.format_duration()
                );
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Library ({} songs) ", self.songs.len()))
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("▶ ");

        // Use borrow_mut() to get mutable access to list_state
        f.render_stateful_widget(list, area, &mut *self.list_state.borrow_mut());
    }

    fn draw_now_playing(&self, f: &mut Frame, area: Rect) {
        let content = if let Some(song) = &self.current_song {
            let status = if self.is_paused { "⏸ PAUSED" } else { "▶ PLAYING" };
            vec![
                Line::from(vec![
                    Span::styled(status, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled(&song.title, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        song.artist.as_deref().unwrap_or("Unknown Artist"),
                        Style::default().fg(Color::Cyan)
                    ),
                    Span::raw(" • "),
                    Span::styled(
                        song.album.as_deref().unwrap_or("Unknown Album"),
                        Style::default().fg(Color::Magenta)
                    ),
                ]),
            ]
        } else {
            vec![Line::from("No song playing")]
        };

        let paragraph = Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title(" Now Playing "));
        f.render_widget(paragraph, area);
    }

    fn draw_controls(&self, f: &mut Frame, area: Rect) {
        let controls = Paragraph::new(vec![
            Line::from(vec![
                Span::raw("↑/↓: Navigate • "),
                Span::raw("Enter: Play • "),
                Span::raw("Space: Pause/Play • "),
                Span::raw("n: Next • "),
                Span::raw("b: Previous • "),
                Span::raw("q: Quit"),
            ])
        ])
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(controls, area);
    }

    fn process_key(&mut self, key: KeyEvent) -> TuiEvent {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => TuiEvent::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => TuiEvent::Quit,
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous_song();
                TuiEvent::Navigate
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_song();
                TuiEvent::Navigate
            }
            KeyCode::Enter => TuiEvent::PlaySelected,
            KeyCode::Char(' ') | KeyCode::Char('p') => TuiEvent::TogglePause,
            KeyCode::Char('n') | KeyCode::Right => TuiEvent::NextTrack,
            KeyCode::Char('b') | KeyCode::Left => TuiEvent::PreviousTrack,
            _ => TuiEvent::None,
        }
    }
}

impl Ui for TuiUi {
    fn show_status(&mut self, is_paused: bool, song: &Song) {
        self.is_paused = is_paused;
        self.current_song = Some(song.clone());
        self.render().ok();
    }

    fn clear_status(&mut self) {
        self.current_song = None;
        self.render().ok();
    }

    fn print_message(&mut self, message: &str) {
        self.status_message = message.to_string();
        self.render().ok();
    }

    fn print_error(&mut self, message: &str) {
        self.status_message = format!("ERROR: {}", message);
        self.render().ok();
    }
}

pub enum TuiEvent {
    Quit,
    PlaySelected,
    TogglePause,
    NextTrack,
    PreviousTrack,
    Navigate,
    None,
}