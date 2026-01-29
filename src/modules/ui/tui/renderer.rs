use crate::application::state::UiState;
use crate::core::events::UiEvent;
use crate::core::traits::UiRenderer;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
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
use std::cell::RefCell;
use std::io::{stdout, Stdout};
use std::time::Duration;
use crate::utils::APP_NAME;

pub struct TuiRenderer {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    list_state: RefCell<ListState>,
    songs: Vec<crate::core::models::Song>,
    current_song: Option<crate::core::models::Song>,
    is_paused: bool,
}

impl TuiRenderer {
    pub fn new() -> Self {
        Self {
            terminal: None,
            list_state: RefCell::new(ListState::default()),
            songs: Vec::new(),
            current_song: None,
            is_paused: false,
        }
    }

    pub fn set_songs(&mut self, songs: Vec<crate::core::models::Song>) {
        self.songs = songs;
        if !self.songs.is_empty() && self.list_state.borrow().selected().is_none() {
            self.list_state.borrow_mut().select(Some(0));
        }
    }

    pub fn update_from_app_state(&mut self, app_state: &crate::application::state::AppState) {
        // Update playback state
        self.current_song = app_state.playback.current_song.clone();
        self.is_paused = app_state.playback.is_paused;

        // Update selected index
        if let Some(index) = app_state.ui.selected_index {
            self.list_state.borrow_mut().select(Some(index));
        }
    }

    fn draw_ui(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(4), // Now playing
                Constraint::Length(3), // Controls
            ])
            .split(f.area());

        self.draw_header(f, chunks[0]);
        self.draw_song_list(f, chunks[1]);
        self.draw_now_playing(f, chunks[2]);
        self.draw_controls(f, chunks[3]);
    }
    
    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new(format!("♪ {} Player ♪", APP_NAME))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
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
                    .title(format!(" Library ({} songs) ", self.songs.len())),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut *self.list_state.borrow_mut());
    }

    fn draw_now_playing(&self, f: &mut Frame, area: Rect) {
        let content = if let Some(song) = &self.current_song {
            let status = if self.is_paused {
                "⏸ PAUSED"
            } else {
                "▶ PLAYING"
            };
            vec![
                Line::from(vec![
                    Span::styled(
                        status,
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(&song.title, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        song.artist.as_deref().unwrap_or("Unknown Artist"),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(" • "),
                    Span::styled(
                        song.album.as_deref().unwrap_or("Unknown Album"),
                        Style::default().fg(Color::Magenta),
                    ),
                ]),
            ]
        } else {
            vec![Line::from("No song playing")]
        };

        let paragraph = Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Now Playing "),
        );
        f.render_widget(paragraph, area);
    }

    fn draw_controls(&self, f: &mut Frame, area: Rect) {
        let controls = Paragraph::new(vec![Line::from(vec![
            Span::raw("↑/↓: Navigate • "),
            Span::raw("Enter: Play • "),
            Span::raw("Space: Pause/Play • "),
            Span::raw("n: Next • "),
            Span::raw("b: Previous • "),
            Span::raw("q: Quit"),
        ])])
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(controls, area);
    }

    fn navigate_up(&mut self) {
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

    fn navigate_down(&mut self) {
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
}

impl UiRenderer for TuiRenderer {
    fn init(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        self.terminal = Some(Terminal::new(backend)?);
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        if let Some(mut terminal) = self.terminal.take() {
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
        }
        Ok(())
    }

    fn render(&mut self, _state: &UiState) -> Result<()> {
        // Take the terminal out temporarily to avoid borrow conflicts
        let mut terminal = match self.terminal.take() {
            Some(t) => t,
            None => return Ok(()),
        };

        terminal.draw(|f| self.draw_ui(f))?;

        // Put the terminal back
        self.terminal = Some(terminal);
        Ok(())
    }

    fn poll_input(&mut self) -> Result<Vec<UiEvent>> {
        let mut events = Vec::new();

        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        events.push(UiEvent::QuitRequested);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        events.push(UiEvent::QuitRequested);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.navigate_up();
                        if let Some(index) = self.list_state.borrow().selected() {
                            events.push(UiEvent::SelectionChanged { index });
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.navigate_down();
                        if let Some(index) = self.list_state.borrow().selected() {
                            events.push(UiEvent::SelectionChanged { index });
                        }
                    }
                    KeyCode::Enter => {
                        events.push(UiEvent::PlaySelectedRequested);
                    }
                    KeyCode::Char(' ') | KeyCode::Char('p') => {
                        events.push(UiEvent::TogglePauseRequested);
                    }
                    KeyCode::Char('n') | KeyCode::Right => {
                        events.push(UiEvent::NextTrackRequested);
                    }
                    KeyCode::Char('b') | KeyCode::Left => {
                        events.push(UiEvent::PreviousTrackRequested);
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
