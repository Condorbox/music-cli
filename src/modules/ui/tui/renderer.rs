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

    // Display state (synced from AppState)
    songs: Vec<crate::core::models::Song>,
    current_song: Option<crate::core::models::Song>,
    is_paused: bool,
    search_active: bool,
    search_query: String,
    search_results: Vec<(usize, crate::core::models::Song)>,
    shuffle: bool,

    // Settings modal state (UI-only)
    show_settings: bool,
    settings_selected: SettingsField,
    temp_volume: u8,
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
            search_active: false,
            search_query: String::new(),
            search_results: Vec::new(),
            shuffle: false,
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
        // Sync playback state
        self.current_song = app_state.playback.current_song.clone();
        self.is_paused = app_state.playback.is_paused;

        // Sync search state from AppState
        self.search_active = app_state.ui.search_active;
        self.search_query = app_state.ui.search_query.clone();
        self.search_results = app_state.ui.search_results.clone();

        self.shuffle = app_state.config.shuffle;

        // Update selected index
        if let Some(index) = app_state.ui.selected_index {
            // Map to display index (search results or full list)
            if self.search_active && !self.search_results.is_empty() {
                // Find position in search results
                if let Some(pos) = self.search_results.iter().position(|(orig_idx, _)| *orig_idx == index) {
                    self.list_state.borrow_mut().select(Some(pos));
                }
            } else {
                self.list_state.borrow_mut().select(Some(index));
            }
        }

        // Update temp volume
        if !self.editing_field || self.settings_selected != SettingsField::Volume {
            self.temp_volume = amplitude_to_volume(app_state.config.volume);
        }
    }

    fn draw_ui(&self, f: &mut Frame) {
        let base_constraints = if self.search_active {
            vec![
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(4), // Now playing
                Constraint::Length(3), // Search bar
            ]
        } else {
            vec![
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(4), // Now playing
                Constraint::Length(3), // Controls
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(base_constraints)
            .split(f.area());

        self.draw_header(f, chunks[0]);
        self.draw_song_list(f, chunks[1]);
        self.draw_now_playing(f, chunks[2]);

        if self.search_active {
            self.draw_search_bar(f, chunks[3]);
        } else {
            self.draw_controls(f, chunks[3]);
        }

        if self.show_settings {
            self.draw_settings_modal(f);
        }
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let title_text = if self.search_active {
            format!("♪ {} Player ♪ - SEARCH MODE", APP_NAME)
        } else {
            format!("♪ {} Player ♪", APP_NAME)
        };

        let title = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(if self.search_active { Color::Yellow } else { Color::Cyan })
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn draw_song_list(&self, f: &mut Frame, area: Rect) {
        let (items, total_count, match_info): (Vec<ListItem>, usize, String) = if self.search_active {
            let items: Vec<ListItem> = self
                .search_results
                .iter()
                .map(|(original_idx, song)| {
                    let content = format!(
                        "[{}] {} - {} [{}]",
                        original_idx + 1,
                        song.artist.as_deref().unwrap_or("Unknown"),
                        song.title,
                        song.format_duration()
                    );
                    ListItem::new(content)
                })
                .collect();

            let match_count = self.search_results.len();
            let match_info = if match_count == 0 {
                " - No matches".to_string()
            } else {
                format!(" - {} match{}", match_count, if match_count == 1 { "" } else { "es" })
            };

            (items, self.songs.len(), match_info)
        } else {
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

            (items, self.songs.len(), String::new())
        };

        let list_title = format!(" Library ({} songs{}) ", total_count, match_info);

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(list_title),
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

            let shuffle_indicator = if self.shuffle {
                " 🔀"
            } else {
                ""
            };

            vec![
                Line::from(vec![
                    Span::styled(
                        status,
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        shuffle_indicator,
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw("  "),
                    Span::styled(&song.title, Style::default().fg(Color::Yellow)),
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
            Span::styled("r: Shuffle • ", Style::default().fg(Color::Cyan)),
            Span::styled("/: Search • ", Style::default().fg(Color::Yellow)),
            Span::raw("s: Settings • "),
            Span::raw("q: Quit"),
        ])])
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(controls, area);
    }

    fn draw_search_bar(&self, f: &mut Frame, area: Rect) {
        let search_text = vec![
            Line::from(vec![
                Span::styled("Search: ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.search_query, Style::default().fg(Color::White)),
                Span::styled("█", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::raw("Esc: Clear • "),
                Span::raw("Enter: Play • "),
                Span::raw("↑/↓: Navigate • "),
                Span::raw("Backspace: Delete • "),
                Span::raw("Ctrl+U: Clear All"),
            ]),
        ];

        let paragraph = Paragraph::new(search_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title(" Search Mode "));
        f.render_widget(paragraph, area);
    }

    fn draw_settings_modal(&self, f: &mut Frame) {
        let area = centered_rect(60, 40, f.area());
        f.render_widget(Clear, area);

        let block = Block::default()
            .title(" ⚙ Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        f.render_widget(block, area);

        let inner_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(4),
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(inner_area);

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

        f.render_widget(Paragraph::new(volume_text).style(volume_style), chunks[0]);

        let path_style = if self.settings_selected == SettingsField::MusicPath {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        f.render_widget(
            Paragraph::new("Music Path: [Press Enter to change] (Coming soon)").style(path_style),
            chunks[1]
        );

        let help_text = if self.editing_field {
            "←/→: Adjust • 0-9: Type value • Enter: Confirm • Esc: Cancel"
        } else {
            "↑/↓: Navigate • Enter: Edit • s/Esc: Close"
        };

        f.render_widget(
            Paragraph::new(help_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            chunks[3]
        );
    }

    fn navigate_up(&mut self) -> Option<usize> {
        let max_len = if self.search_active {
            self.search_results.len()
        } else {
            self.songs.len()
        };

        if max_len == 0 {
            return None;
        }

        let mut state = self.list_state.borrow_mut();
        let new_idx = match state.selected() {
            Some(i) => {
                if i == 0 {
                    max_len.saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(new_idx));

        // Return original index for event
        self.get_original_index(new_idx)
    }

    fn navigate_down(&mut self) -> Option<usize> {
        let max_len = if self.search_active {
            self.search_results.len()
        } else {
            self.songs.len()
        };

        if max_len == 0 {
            return None;
        }

        let mut state = self.list_state.borrow_mut();
        let new_idx = match state.selected() {
            Some(i) => {
                if i >= max_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        state.select(Some(new_idx));

        // Return original index for event
        self.get_original_index(new_idx)
    }

    fn get_original_index(&self, display_idx: usize) -> Option<usize> {
        if self.search_active {
            self.search_results.get(display_idx).map(|(orig_idx, _)| *orig_idx)
        } else {
            Some(display_idx)
        }
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
        let mut terminal = match self.terminal.take() {
            Some(t) => t,
            None => return Ok(()),
        };

        terminal.draw(|f| self.draw_ui(f))?;
        self.terminal = Some(terminal);
        Ok(())
    }

    fn poll_input(&mut self) -> Result<Vec<UiEvent>> {
        let mut events = Vec::new();

        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                // Settings modal takes priority
                if self.show_settings {
                    if self.editing_field {
                        match key.code {
                            KeyCode::Enter => {
                                self.editing_field = false;
                                if self.settings_selected == SettingsField::Volume {
                                    events.push(UiEvent::VolumeChangeRequested {
                                        volume: self.temp_volume
                                    });
                                }
                            }
                            KeyCode::Esc => {
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
                }
                // Search mode
                else if self.search_active {
                    match key.code {
                        KeyCode::Esc => {
                            // Exit search mode - emit event
                            events.push(UiEvent::SearchToggled { active: false });
                        }
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Clear search query - emit event
                            events.push(UiEvent::SearchQueryChanged {
                                query: String::new()
                            });
                        }
                        KeyCode::Backspace => {
                            // Delete last character - emit event with new query
                            let mut new_query = self.search_query.clone();
                            new_query.pop();
                            events.push(UiEvent::SearchQueryChanged {
                                query: new_query
                            });
                        }
                        KeyCode::Up => {
                            if let Some(index) = self.navigate_up() {
                                events.push(UiEvent::SelectionChanged { index });
                            }
                        }
                        KeyCode::Down => {
                            if let Some(index) = self.navigate_down() {
                                events.push(UiEvent::SelectionChanged { index });
                            }
                        }
                        KeyCode::Enter => {
                            events.push(UiEvent::PlaySelectedRequested);
                        }
                        KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            events.push(UiEvent::TogglePauseRequested);
                        }
                        KeyCode::Char(c) => {
                            // Add character - emit event with new query
                            let mut new_query = self.search_query.clone();
                            new_query.push(c);
                            events.push(UiEvent::SearchQueryChanged {
                                query: new_query
                            });
                        }
                        _ => {}
                    }
                }
                // Normal mode
                else {
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
                        KeyCode::Char('/') => {
                            if !self.songs.is_empty() {
                                events.push(UiEvent::SearchToggled { active: true });
                            }
                        }
                        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if !self.songs.is_empty() {
                                events.push(UiEvent::SearchToggled { active: true });
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if let Some(index) = self.navigate_up() {
                                events.push(UiEvent::SelectionChanged { index });
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let Some(index) = self.navigate_down() {
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
                        KeyCode::Char('r') => {
                            events.push(UiEvent::ShuffleToggled{shuffle_enabled: self.shuffle});
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