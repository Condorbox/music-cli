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
use std::sync::Arc;
use std::time::Duration;
use crate::modules::library::sorter::SortField;
use crate::modules::playback::playback_progress::PlaybackProgress;
use crate::modules::ui::tui::settings_state::{PathValidation, SettingsField, SettingsState};
use crate::utils::{
    repeat_label, APP_NAME, MIN_TRUNCATE_FIELD, MIN_TRUNCATE_TITLE,
};

pub struct TuiRenderer {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    list_state: RefCell<ListState>,

    // Display state (synced from AppState)
    songs: Arc<Vec<crate::core::models::Song>>,
    current_song: Option<crate::core::models::Song>,
    current_elapsed: Duration, // Synced from AppState.playback.current_elapsed
    is_paused: bool,
    search_active: bool,
    search_query: String,
    search_results: Vec<(usize, crate::core::models::Song)>,
    shuffle: bool,

    settings: SettingsState,

    active_sort: Option<SortField>,
}

impl TuiRenderer {
    pub fn new() -> Self {
        Self {
            terminal: None,
            list_state: RefCell::new(ListState::default()),
            songs: Arc::new(Vec::new()),
            current_song: None,
            is_paused: false,
            search_active: false,
            search_query: String::new(),
            search_results: Vec::new(),
            shuffle: false,
            current_elapsed: Duration::from_secs(0),
            settings: SettingsState::default(),
            active_sort: None,
        }
    }

    pub fn set_songs(&mut self, songs: Arc<Vec<crate::core::models::Song>>) {
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

        if self.settings.is_open() {
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
        let current_path = self.current_song.as_ref().map(|s| &s.path);
        // 2 border chars + 2 highlight-symbol chars ("▶ ")
        let content_width = area.width.saturating_sub(4);

        let (items, total_count, match_info): (Vec<ListItem>, usize, String) = if self.search_active {
            let items: Vec<ListItem> = self
                .search_results
                .iter()
                .map(|(_original_idx, song)| {
                    let is_current = current_path.is_some_and(|p| p == &song.path);
                    song_list_item(None, song, is_current, content_width)
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
                    let is_current = current_path.is_some_and(|p| p == &song.path);
                    song_list_item(Some(i + 1), song, is_current, content_width)
                })
                .collect();

            (items, self.songs.len(), String::new())
        };

        let sort_label = if self.search_active { "" } else { active_sort_label(self.active_sort) };
        let list_title = format!(" Library ({} songs{}) {} ", total_count, match_info, sort_label);

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(list_title))
            .highlight_style(
                // DarkGray bg is kept (user's preference).
                // BOLD ensures all span text punches through the background.
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
                        song.format_artists(),
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
            Span::styled("F5: Refresh • ", Style::default().fg(Color::Green)),
            Span::raw("s: Settings • "),
            Span::raw("o: Sort • "),
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
        // Make the modal taller when path editing is active so the error line fits.
        let height_pct = if self.settings.is_editing_path() { 60 } else { 50 };
        let area = centered_rect(60, height_pct, f.area());
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

        // Extra row for the inline path error when needed.
        let path_error_height = match self.settings.path_validation() {
            PathValidation::Error(_) => 1,
            PathValidation::Idle => 0,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                          // Volume
                Constraint::Length(3),                          // Repeat
                Constraint::Length(3),                          // Music Path input
                Constraint::Length(path_error_height),          // Inline error (0 or 1)
                Constraint::Min(0),                             // spacer
                Constraint::Length(2),                          // help
            ])
            .split(inner);

        self.draw_settings_volume(f, chunks[0]);
        self.draw_settings_repeat(f, chunks[1]);
        self.draw_settings_path(f, chunks[2]);
        self.draw_settings_path_error(f, chunks[3]);
        self.draw_settings_help(f, chunks[5]);
    }

    fn draw_settings_volume(&self, f: &mut Frame, area: Rect) {
        let selected = self.settings.selected() == SettingsField::Volume;
        let editing = selected && self.settings.is_editing_volume();

        let label = if editing {
            format!(
                "Volume: {}%  [←/→ adjust • 0-9 type • Enter confirm • Esc cancel]",
                self.settings.temp_volume()
            )
        } else {
            format!("Volume: {}%", self.settings.temp_volume())
        };

        f.render_widget(
            Paragraph::new(label).style(field_style(selected)),
            area,
        );
    }

    fn draw_settings_repeat(&self, f: &mut Frame, area: Rect) {
        let selected = self.settings.selected() == SettingsField::Repeat;

        let label = if selected {
            let temp_repeat = self.settings.temp_repeat();
            format!(
                "Repeat: {} {}  [←/→ or Enter to cycle]",
                temp_repeat.symbol(),
                repeat_label(temp_repeat),
            )
        } else {
            let temp_repeat = self.settings.temp_repeat();
            format!(
                "Repeat: {} {}",
                temp_repeat.symbol(),
                repeat_label(temp_repeat),
            )
        };

        f.render_widget(
            Paragraph::new(label).style(field_style(selected)),
            area,
        );
    }

    fn draw_settings_path(&self, f: &mut Frame, area: Rect) {
        let selected = self.settings.selected() == SettingsField::MusicPath;

        // The "label" color drives the key text ("Music Path:") and hint.
        // When selected: yellow. When not: white.
        let label_color = if selected { Color::Yellow } else { Color::White };
        // Hint text is always a dimmer shade of whatever the label color is.
        let hint_color = if selected { Color::Yellow } else { Color::DarkGray };

        let label: Line = if self.settings.is_editing_path() {
            Line::from(vec![
                Span::styled("Music Path: ", Style::default().fg(label_color).add_modifier(Modifier::BOLD)),
                Span::styled(self.settings.temp_path(), Style::default().fg(Color::White)),
                Span::styled("█", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "  [Enter confirm • Esc cancel • Ctrl+U clear]",
                    Style::default().fg(Color::Yellow),
                ),
            ])
        } else if self.settings.temp_path().is_empty() {
            let mut spans = vec![
                Span::styled("Music Path: ", Style::default().fg(label_color)),
                Span::styled("(not set)", Style::default().fg(Color::DarkGray)),
            ];
            if selected {
                spans.push(Span::styled("  [Enter to set]", Style::default().fg(hint_color)));
            }
            Line::from(spans)
        } else {
            let mut spans = vec![
                Span::styled("Music Path: ", Style::default().fg(label_color)),
                Span::styled(self.settings.temp_path(), Style::default().fg(Color::Cyan)),
            ];
            if selected {
                spans.push(Span::styled("  [Enter to change]", Style::default().fg(hint_color)));
            }
            Line::from(spans)
        };

        f.render_widget(Paragraph::new(label), area);
    }

    /// Renders the inline error line directly below the path field.
    /// Renders nothing (zero-height) when there is no error.
    fn draw_settings_path_error(&self, f: &mut Frame, area: Rect) {
        if let PathValidation::Error(msg) = self.settings.path_validation() {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("  ✗ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::styled(msg.as_str(), Style::default().fg(Color::Red)),
                ])),
                area,
            );
        }
    }

    fn draw_settings_help(&self, f: &mut Frame, area: Rect) {
        let text = if self.settings.is_editing_volume() {
            "←/→: Adjust  •  0-9: Type value  •  Enter: Confirm  •  Esc: Cancel"
        } else if self.settings.is_editing_path() {
            "Type path  •  Enter: Confirm  •  Esc: Cancel  •  Ctrl+U: Clear"
        } else {
            match self.settings.selected() {
                SettingsField::Volume =>
                    "↑/↓: Navigate  •  Enter: Edit volume  •  s/Esc: Close",
                SettingsField::Repeat =>
                    "↑/↓: Navigate  •  ←/→ or Enter: Cycle mode  •  s/Esc: Close",
                SettingsField::MusicPath =>
                    "↑/↓: Navigate  •  Enter: Edit path  •  s/Esc: Close",
            }
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

        if self.settings.is_open() {
            events.extend(self.settings.handle_key(key));
        } else if self.search_active {
            self.handle_search_input(key, &mut events);
        } else {
            self.handle_normal_input(key, &mut events);
        }

        Ok(events)
    }

    fn update_state(&mut self, app_state: &crate::application::state::AppState) {
        // Sync playback state
        self.songs = Arc::clone(&app_state.library.songs);  // Arc::clone so O(1)
        self.current_song = app_state.playback.current_song.clone();
        self.current_elapsed = app_state.playback.current_elapsed;
        self.is_paused = app_state.playback.is_paused;

        // Sync search state from AppState
        self.search_active = app_state.ui.search_active;
        self.search_query = app_state.ui.search_query.clone();
        self.search_results = app_state.ui.search_results.clone();

        // Sync shuffle state
        self.shuffle = app_state.config.shuffle;
        self.settings.sync_from_app_state(app_state);

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

        self.active_sort    = app_state.library.active_sort;
    }
}

impl TuiRenderer {
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
                self.settings.open();
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
            KeyCode::F(5) | KeyCode::Char('u')=> {
                events.push(UiEvent::RefreshRequested);
            }
            KeyCode::Char('o') => {
                events.push(UiEvent::SortCycleRequested);
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

fn active_sort_label(active_sort: Option<SortField>) -> &'static str {
    match active_sort {
        None                      => "",
        Some(SortField::Title)    => "[↑ title]",
        Some(SortField::Artist)   => "[↑ artist]",
        Some(SortField::Album)    => "[↑ album]",
        Some(SortField::Duration) => "[↑ duration]",
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

fn truncate_str(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let mut chars = s.chars();
    let mut out = String::with_capacity(max_chars);
    for _ in 0..max_chars {
        match chars.next() {
            None => return out,     // fits in full
            Some(c) => out.push(c),
        }
    }
    // There are still characters left, we truncated.
    out.pop();
    out.push('…');
    out
}


fn song_list_item(num: Option<usize>, song: &crate::core::models::Song, is_current: bool, available_width: u16) -> ListItem<'static> {
    const SEP: &str = "  ·  ";       // 5 chars
    const INDEX_WIDTH: usize = 6;    // "  1.  "
    const DURATION_WIDTH: usize = 10; // "  [59:59]" worst case

    let has_artist = !song.artists.is_empty();
    let has_album  = song.album.is_some();

    let sep_count = has_artist as usize + has_album as usize;
    let dur_width = if song.duration.is_some() { DURATION_WIDTH } else { 0 };

    let text_space = (available_width as usize)
        .saturating_sub(INDEX_WIDTH)
        .saturating_sub(dur_width)
        .saturating_sub(sep_count * SEP.len());

    // Percentage split of the remaining text space
    let (title_max, artist_max, album_max) = match (has_artist, has_album) {
        (false, false) => (text_space, 0, 0),
        (true,  false) => (text_space * 55 / 100, text_space * 45 / 100, 0),
        (false, true)  => (text_space * 60 / 100, 0, text_space * 40 / 100),
        (true,  true)  => (text_space * 45 / 100, text_space * 35 / 100, text_space * 20 / 100),
    };

    // Clamp: never truncate to fewer than MIN_TRUNCATE_TITLE for title /
    // MIN_TRUNCATE_FIELD for others. Fields that can't even fit
    // MIN_TRUNCATE_FIELD chars are omitted entirely.
    let title = truncate_str(&song.title, title_max.max(MIN_TRUNCATE_TITLE));
    let artists_str = song.format_artists();
    let artist = (has_artist && artist_max >= MIN_TRUNCATE_FIELD)
        .then(|| truncate_str(&artists_str, artist_max));
    let album = song.album.as_ref()
        .filter(|_| album_max >= MIN_TRUNCATE_FIELD)
        .map(|a| truncate_str(a, album_max));

    // ── Styles ────────────────────────────────────────────────────────────
    // When the song is currently playing every colored element turns green
    // The index and duration are always Gray — structural, not content
    let (title_style, artist_style, album_style, sep_style) = if is_current {
        (
            Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Green),
            Style::default().fg(Color::Green),
            Style::default().fg(Color::Green),
        )
    } else {
        (
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Cyan),
            Style::default().fg(Color::LightBlue),
            Style::default().fg(Color::Gray),
        )
    };

    // Gray for structural/positional text; visible on both black bg and
    // DarkGray highlight bg without clashing with content colors
    let structural = Style::default().fg(Color::Gray);

    // ── Assemble ─────────────────────────────────────────────────────────
    let mut spans: Vec<Span> = Vec::with_capacity(9);

    match num {
        Some(n) => spans.push(Span::styled(format!("{:3}.  ", n), structural)),
        None    => spans.push(Span::raw("      ")),   // 6 spaces
    }
    spans.push(Span::styled(title, title_style));

    if let Some(a) = artist {
        spans.push(Span::styled(SEP, sep_style));
        spans.push(Span::styled(a, artist_style));
    }

    if let Some(al) = album {
        spans.push(Span::styled(SEP, sep_style));
        spans.push(Span::styled(al, album_style));
    }

    if song.duration.is_some() {
        spans.push(Span::styled(
            format!("  [{}]", song.format_duration()),
            structural,
        ));
    }

    ListItem::new(Line::from(spans))
}
