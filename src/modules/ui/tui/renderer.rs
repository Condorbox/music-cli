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
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::cell::RefCell;
use std::io::{stdout, Stdout};
use std::time::Duration;
use crate::utils::{amplitude_to_volume, APP_NAME};

#[derive(Debug, Clone, Copy, PartialEq)]
enum SettingsField {
    MusicPath,
    Volume,
}

pub struct TuiRenderer {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    list_state: RefCell<ListState>,
    songs: Vec<crate::core::models::Song>,
    current_song: Option<crate::core::models::Song>,
    is_paused: bool,

    // Settings modal state
    show_settings: bool,
    settings_selected: SettingsField,
    temp_volume: u8,  // Temporary value while editing
    editing_field: bool,
}

impl TuiRenderer {
    pub fn new() -> Self {
        Self {
            terminal: None,
            list_state: RefCell::new(ListState::default()),
            songs: Vec::new(),
            current_song: None,
            is_paused: false,
            show_settings: false,
            settings_selected: SettingsField::Volume,
            temp_volume: 100,
            editing_field: false,
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

        // Update temp volume from config
        self.temp_volume = amplitude_to_volume(app_state.config.volume);
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

        // Draw settings modal on top if active
        if self.show_settings {
            self.draw_settings_modal(f);
        }
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
            Span::raw("s: Settings • "),
            Span::raw("q: Quit"),
        ])])
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(controls, area);
    }

    fn draw_settings_modal(&self, f: &mut Frame) {
        // Create centered modal
        let area = centered_rect(60, 40, f.area());

        // Clear the background
        f.render_widget(Clear, area);

        let block = Block::default()
            .title(" ⚙ Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        f.render_widget(block, area);

        // Inner area for content
        let inner_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(4),
        };

        // Split into sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Volume
                Constraint::Length(3), // Path (future)
                Constraint::Min(0),    // Spacer
                Constraint::Length(2), // Help text
            ])
            .split(inner_area);

        // Volume setting
        let volume_style = if self.settings_selected == SettingsField::Volume {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let volume_text = if self.editing_field && self.settings_selected == SettingsField::Volume {
            format!("Volume: {}% [EDITING - Use ←/→ or 0-9, Enter to confirm]", self.temp_volume)
        } else {
            format!("Volume: {}%", self.temp_volume)
        };

        let volume_widget = Paragraph::new(volume_text)
            .style(volume_style);
        f.render_widget(volume_widget, chunks[0]);

        // Music path setting
        let path_style = if self.settings_selected == SettingsField::MusicPath {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let path_widget = Paragraph::new("Music Path: [Press Enter to change] (Coming soon)")
            .style(path_style);
        f.render_widget(path_widget, chunks[1]);

        // Help text
        let help_text = if self.editing_field {
            "←/→: Adjust • 0-9: Type value • Enter: Confirm • Esc: Cancel"
        } else {
            "↑/↓: Navigate • Enter: Edit • s/Esc: Close"
        };

        let help_widget = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(help_widget, chunks[3]);
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

    fn settings_navigate_up(&mut self) {
        self.settings_selected = match self.settings_selected {
            SettingsField::Volume => SettingsField::MusicPath,
            SettingsField::MusicPath => SettingsField::Volume,
        };
    }

    fn settings_navigate_down(&mut self) {
        self.settings_selected = match self.settings_selected {
            SettingsField::Volume => SettingsField::MusicPath,
            SettingsField::MusicPath => SettingsField::Volume,
        };
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
                // Handle settings modal input
                if self.show_settings {
                    if self.editing_field {
                        // Editing mode
                        match key.code {
                            KeyCode::Enter => {
                                // Confirm the change
                                self.editing_field = false;
                                match self.settings_selected {
                                    SettingsField::Volume => {
                                        events.push(UiEvent::VolumeChangeRequested {
                                            volume: self.temp_volume
                                        });
                                    }
                                    SettingsField::MusicPath => {
                                        // TODO: Implement path change

                                    }
                                }
                            }
                            KeyCode::Esc => {
                                // Cancel editing
                                self.editing_field = false;
                            }
                            KeyCode::Left => {
                                if self.settings_selected == SettingsField::Volume {
                                    self.temp_volume = self.temp_volume.saturating_sub(5);
                                }
                            }
                            KeyCode::Right => {
                                if self.settings_selected == SettingsField::Volume {
                                    self.temp_volume = (self.temp_volume + 5).min(100);
                                }
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() => {
                                // Allow typing volume value
                                if self.settings_selected == SettingsField::Volume {
                                    let digit = c.to_digit(10).unwrap() as u8;
                                    let new_val = (self.temp_volume % 10) * 10 + digit;
                                    if new_val <= 100 {
                                        self.temp_volume = new_val;
                                    }
                                }
                            }
                            _ => {}
                        }
                    } else {
                        // Navigation mode in settings
                        match key.code {
                            KeyCode::Char('s') | KeyCode::Esc => {
                                self.show_settings = false;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                self.settings_navigate_up();
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.settings_navigate_down();
                            }
                            KeyCode::Enter => {
                                self.editing_field = true;
                            }
                            _ => {}
                        }
                    }
                } else {
                    // Normal mode
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            events.push(UiEvent::QuitRequested);
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            events.push(UiEvent::QuitRequested);
                        }
                        KeyCode::Char('s') => {
                            self.show_settings = true;
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
        }

        Ok(events)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// Helper function to create centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}