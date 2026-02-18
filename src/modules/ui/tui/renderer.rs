use crate::application::state::UiState;
use crate::core::events::UiEvent;
use crate::core::traits::UiRenderer;
use crate::modules::ui::progress_formatter::format_duration;
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
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::cell::RefCell;
use std::io::{stdout, Stdout};
use std::time::Duration;
use crate::core::models::RepeatMode;
use crate::modules::playback::playback_progress::PlaybackProgress;
use crate::utils::{amplitude_to_volume, APP_NAME};

const SETTINGS_FIELDS: &[SettingsField] = &[
    SettingsField::Volume,
    SettingsField::Repeat,
    SettingsField::MusicPath,
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum SettingsField {
    MusicPath,
    Volume,
    Repeat
}

pub struct TuiRenderer {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    list_state: RefCell<ListState>,

    // Display state (synced from AppState)
    songs: Vec<crate::core::models::Song>,
    current_song: Option<crate::core::models::Song>,
    current_elapsed: Duration, // Synced from AppState.playback.current_elapsed
    is_paused: bool,
    search_active: bool,
    search_query: String,
    search_results: Vec<(usize, crate::core::models::Song)>,
    shuffle: bool,

    // Settings modal state (UI-only)
    show_settings: bool,
    settings_selected: SettingsField,
    temp_volume: u8,
    temp_repeat: RepeatMode,
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
            current_elapsed: Duration::from_secs(0),
            show_settings: false,
            settings_selected: SettingsField::Volume,
            temp_volume: 100,
            temp_repeat: RepeatMode::default(),
            editing_field: false,
        }
    }

    pub fn set_songs(&mut self, songs: Vec<crate::core::models::Song>) {
        self.songs = songs;
        if !self.songs.is_empty() && self.list_state.borrow().selected().is_none() {
            self.list_state.borrow_mut().select(Some(0));
        }
    }

    fn draw_ui(&self, f: &mut Frame) {
        let base_constraints = if self.search_active {
            vec![
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(5), // Now playing (with progress bar)
                Constraint::Length(3), // Search bar
            ]
        } else {
            vec![
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(5), // Now playing (with progress bar)
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
        // Create the main block container
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Now Playing ");

        // Calculate the inner area (inside the borders)
        let inner_area = block.inner(area);

        // Render the block borders first
        f.render_widget(block, area);

        // Split the inner area: Top for Song Info, Bottom for Progress Bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Text takes remaining space
                Constraint::Length(1), // Progress bar takes 1 line
            ])
            .split(inner_area);

        // Logic for Song Info (Top Chunk)
        if let Some(song) = &self.current_song {
            let status = if self.is_paused {
                "⏸ PAUSED"
            } else {
                "▶ PLAYING"
            };

            let shuffle_indicator = if self.shuffle {
                " 🔀"
            } else {
                " ▶️"
            };

            let text_content = vec![
                Line::from(vec![
                    Span::styled(
                        status,
                        Style::default()
                            .fg(if self.is_paused { Color::Yellow } else { Color::Green })
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        shuffle_indicator,
                        Style::default().fg(Color::Cyan),
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
            ];

            f.render_widget(Paragraph::new(text_content), chunks[0]);

            // Spotify-style Progress Bar (Bottom Chunk): [elapsed] [bar] [total]
            if let Some(duration) = song.duration {
                if let Some(progress) = PlaybackProgress::new(self.current_elapsed, duration) {
                    let elapsed_str = format_duration(progress.elapsed());
                    let total_str = format_duration(progress.total());

                    // Split horizontally: elapsed | padding | bar | padding | total
                    let progress_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(elapsed_str.len() as u16), // Elapsed time
                            Constraint::Length(1),                         // Left padding
                            Constraint::Min(1),                            // Bar takes remaining
                            Constraint::Length(1),                         // Right padding
                            Constraint::Length(total_str.len() as u16),  // Total time
                        ])
                        .split(chunks[1]);

                    // Elapsed time (left)
                    let elapsed_widget = Paragraph::new(elapsed_str)
                        .style(Style::default().fg(Color::White));
                    f.render_widget(elapsed_widget, progress_chunks[0]);

                    // Progress bar (center) - NO LABEL, just the bar
                    let gauge = Gauge::default()
                        .gauge_style(Style::default().fg(Color::LightBlue).bg(Color::DarkGray))
                        .ratio(progress.ratio())
                        .use_unicode(true)
                        .label(""); // No percentage
                    f.render_widget(gauge, progress_chunks[2]);

                    // Total time (right)
                    let total_widget = Paragraph::new(total_str)
                        .style(Style::default().fg(Color::Gray));
                    f.render_widget(total_widget, progress_chunks[4]);
                }
            }
        } else {
            f.render_widget(Paragraph::new(vec![Line::from("No song playing")]), chunks[0]);
        }
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
        let area = centered_rect(60, 50, f.area());
        f.render_widget(Clear, area);
        f.render_widget(
            Block::default()
                .title(" ⚙ Settings ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
            area,
        );

        let inner = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(4),
        };

        // One row per settings field + a spacer + help line.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Volume
                Constraint::Length(3), // Repeat
                Constraint::Length(3), // Music Path
                Constraint::Min(0),    // spacer
                Constraint::Length(2), // help
            ])
            .split(inner);

        self.draw_settings_volume(f, chunks[0]);
        self.draw_settings_repeat(f, chunks[1]);
        self.draw_settings_path(f, chunks[2]);
        self.draw_settings_help(f, chunks[4]);
    }

    fn draw_settings_volume(&self, f: &mut Frame, area: Rect) {
        let selected = self.settings_selected == SettingsField::Volume;
        let editing = selected && self.editing_field;

        let label = if editing {
            format!("Volume: {}%  [←/→ adjust • 0-9 type • Enter confirm • Esc cancel]", self.temp_volume)
        } else {
            format!("Volume: {}%", self.temp_volume)
        };

        f.render_widget(
            Paragraph::new(label).style(field_style(selected)),
            area,
        );
    }

    fn draw_settings_repeat(&self, f: &mut Frame, area: Rect) {
        let selected = self.settings_selected == SettingsField::Repeat;

        let label = if selected {
            format!(
                "Repeat: {} {}  [←/→ or Enter to cycle]",
                self.temp_repeat.symbol(),
                repeat_label(self.temp_repeat),
            )
        } else {
            format!(
                "Repeat: {} {}",
                self.temp_repeat.symbol(),
                repeat_label(self.temp_repeat),
            )
        };

        f.render_widget(
            Paragraph::new(label).style(field_style(selected)),
            area,
        );
    }

    fn draw_settings_path(&self, f: &mut Frame, area: Rect) {
        let selected = self.settings_selected == SettingsField::MusicPath;
        f.render_widget(
            Paragraph::new("Music Path: [coming soon]").style(field_style(selected)),
            area,
        );
    }

    fn draw_settings_help(&self, f: &mut Frame, area: Rect) {
        let text = if self.editing_field {
            "←/→: Adjust • 0-9: Type value • Enter: Confirm • Esc: Cancel"
        } else {
            "↑/↓: Navigate fields • Enter: Edit • s/Esc: Close"
        };

        f.render_widget(
            Paragraph::new(text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            area,
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

    fn settings_navigate_up(&mut self) {
        let current = SETTINGS_FIELDS
            .iter()
            .position(|f| *f == self.settings_selected)
            .unwrap_or(0);
        let prev = (current + SETTINGS_FIELDS.len() - 1) % SETTINGS_FIELDS.len();
        self.settings_selected = SETTINGS_FIELDS[prev];
    }

    fn settings_navigate_down(&mut self) {
        let current = SETTINGS_FIELDS
            .iter()
            .position(|f| *f == self.settings_selected)
            .unwrap_or(0);
        let next = (current + 1) % SETTINGS_FIELDS.len();
        self.settings_selected = SETTINGS_FIELDS[next];
    }


    fn get_original_index(&self, display_idx: usize) -> Option<usize> {
        if self.search_active {
            self.search_results.get(display_idx).map(|(orig_idx, _)| *orig_idx)
        } else {
            Some(display_idx)
        }
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
        if let Some(mut terminal) = self.terminal.take() {
            terminal.draw(|f| self.draw_ui(f))?;
            self.terminal = Some(terminal);
        }
        Ok(())
    }

    fn poll_input(&mut self) -> Result<Vec<UiEvent>> {
        let mut events = Vec::new();

        if !event::poll(Duration::from_millis(0))? {
            return Ok(events);
        }

        let Event::Key(key) = event::read()? else {
            return Ok(events);
        };

        if self.show_settings {
            self.handle_settings_input(key, &mut events);
        } else if self.search_active {
            self.handle_search_input(key, &mut events);
        } else {
            self.handle_normal_input(key, &mut events);
        }

        Ok(events)
    }

    fn update_state(&mut self, app_state: &crate::application::state::AppState) {
        // Sync playback state
        self.current_song = app_state.playback.current_song.clone();
        self.current_elapsed = app_state.playback.current_elapsed;
        self.is_paused = app_state.playback.is_paused;

        // Sync search state from AppState
        self.search_active = app_state.ui.search_active;
        self.search_query = app_state.ui.search_query.clone();
        self.search_results = app_state.ui.search_results.clone();

        // Sync shuffle state
        self.shuffle = app_state.config.shuffle;

        // Sync repeat — always safe since it has no confirm-step editing mode.
        self.temp_repeat = app_state.config.repeat;

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

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}


impl TuiRenderer {
    fn handle_settings_input(&mut self, key: event::KeyEvent, events: &mut Vec<UiEvent>) {
        if self.editing_field {
            self.handle_settings_editing_input(key, events);
        } else {
            self.handle_settings_navigation_input(key, events);
        }
    }

    fn handle_settings_editing_input(&mut self, key: event::KeyEvent, events: &mut Vec<UiEvent>) {
        // Only Volume uses confirm-step editing.
        match key.code {
            KeyCode::Enter => {
                self.editing_field = false;
                events.push(UiEvent::VolumeChangeRequested { volume: self.temp_volume });
            }
            KeyCode::Esc => {
                self.editing_field = false;
            }
            KeyCode::Left => {
                self.temp_volume = self.temp_volume.saturating_sub(5);
            }
            KeyCode::Right => {
                self.temp_volume = (self.temp_volume + 5).min(100);
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let digit = c.to_digit(10).unwrap() as u8;
                let new_val = (self.temp_volume % 10) * 10 + digit;
                if new_val <= 100 {
                    self.temp_volume = new_val;
                }
            }
            _ => {}
        }
    }

    fn handle_settings_navigation_input(&mut self, key: event::KeyEvent, events: &mut Vec<UiEvent>) {
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
            KeyCode::Enter => match self.settings_selected {
                SettingsField::Volume => {
                    self.editing_field = true;
                }
                SettingsField::Repeat => {
                    // Enter cycles forward — no confirm step needed.
                    self.temp_repeat = self.temp_repeat.cycle();
                    events.push(UiEvent::RepeatChangeRequested { mode: self.temp_repeat });
                }
                SettingsField::MusicPath => {
                    // TODO Implement it
                }
            },
            KeyCode::Left => {
                if self.settings_selected == SettingsField::Repeat {
                    self.temp_repeat = self.temp_repeat.cycle_back();
                    events.push(UiEvent::RepeatChangeRequested { mode: self.temp_repeat });
                }
            }
            KeyCode::Right => {
                if self.settings_selected == SettingsField::Repeat {
                    self.temp_repeat = self.temp_repeat.cycle();
                    events.push(UiEvent::RepeatChangeRequested { mode: self.temp_repeat });
                }
            }
            _ => {}
        }
    }

    fn handle_search_input(&mut self, key: event::KeyEvent, events: &mut Vec<UiEvent>) {
        match key.code {
            KeyCode::Esc => {
                events.push(UiEvent::SearchToggled { active: false });
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                events.push(UiEvent::SearchQueryChanged { query: String::new() });
            }
            KeyCode::Backspace => {
                let mut q = self.search_query.clone();
                q.pop();
                events.push(UiEvent::SearchQueryChanged { query: q });
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
                let mut q = self.search_query.clone();
                q.push(c);
                events.push(UiEvent::SearchQueryChanged { query: q });
            }
            _ => {}
        }
    }

    fn handle_normal_input(&mut self, key: event::KeyEvent, events: &mut Vec<UiEvent>) {
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
                events.push(UiEvent::ShuffleToggled { shuffle_enabled: self.shuffle });
            }
            _ => {}
        }
    }
}

// Style for a settings field row — highlighted when selected.
fn field_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

fn repeat_label(mode: RepeatMode) -> &'static str {
    match mode {
        RepeatMode::Off => "off",
        RepeatMode::All => "all",
        RepeatMode::One => "one",
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