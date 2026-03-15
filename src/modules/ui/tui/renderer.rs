use crate::application::state::UiState;
use crate::core::events::UiEvent;
use crate::core::traits::UiRenderer;
use crate::modules::input::{map_key, InputAction, InputMode, KeyConfig};
use crate::modules::ui::progress_formatter::format_duration;
use crate::modules::ui::key_hints;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::cell::RefCell;
use std::io::{stdout, Stdout};
use std::sync::Arc;
use std::time::Duration;
use crate::modules::library::sorter::SortField;
use crate::modules::playback::playback_progress::PlaybackProgress;
use crate::modules::ui::tui::settings_state::SettingsState;
use crate::modules::ui::tui::settings_view;
use crate::utils::{
    APP_NAME, MIN_TRUNCATE_FIELD, MIN_TRUNCATE_TITLE,
};

pub struct TuiRenderer {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    list_state: RefCell<ListState>,

    key_config: KeyConfig,
    key_config_synced: bool,

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
            key_config: KeyConfig::default(),
            key_config_synced: false,
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
                Constraint::Length(4), // Search bar (query + help)
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
            settings_view::draw(f, &self.settings, &self.key_config);
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
        let cfg = &self.key_config;

        let nav_up = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::NavigateUp,
            &[key_hints::kb(KeyCode::Up)],
        );
        let nav_down = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::NavigateDown,
            &[key_hints::kb(KeyCode::Down)],
        );
        let play = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::PlaySelected,
            &[key_hints::kb(KeyCode::Enter)],
        );
        let pause = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::TogglePause,
            &[key_hints::kb(KeyCode::Char(' '))],
        );
        let next = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::NextTrack,
            &[key_hints::kb(KeyCode::Char('n'))],
        );
        let prev = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::PreviousTrack,
            &[key_hints::kb(KeyCode::Char('b'))],
        );
        let shuffle = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::ToggleShuffle,
            &[key_hints::kb(KeyCode::Char('r'))],
        );
        let search = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::EnterSearch,
            &[key_hints::kb(KeyCode::Char('/'))],
        );
        let refresh = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::Refresh,
            &[key_hints::kb(KeyCode::F(5))],
        );
        let settings = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::OpenSettings,
            &[key_hints::kb(KeyCode::Char('s'))],
        );
        let sort = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::CycleSort,
            &[key_hints::kb(KeyCode::Char('o'))],
        );
        let quit = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Normal,
            InputAction::Quit,
            &[key_hints::kb(KeyCode::Char('q'))],
        );

        let controls = Paragraph::new(vec![Line::from(vec![
            Span::raw(format!(
                "{}/{}: Navigate • ",
                key_hints::format_binding_opt(nav_up),
                key_hints::format_binding_opt(nav_down)
            )),
            Span::raw(format!(
                "{}: Play • ",
                key_hints::format_binding_opt(play)
            )),
            Span::raw(format!(
                "{}: Pause/Play • ",
                key_hints::format_binding_opt(pause)
            )),
            Span::raw(format!(
                "{}: Next • ",
                key_hints::format_binding_opt(next)
            )),
            Span::raw(format!(
                "{}: Previous • ",
                key_hints::format_binding_opt(prev)
            )),
            Span::styled(
                format!("{}: Shuffle • ", key_hints::format_binding_opt(shuffle)),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("{}: Search • ", key_hints::format_binding_opt(search)),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!("{}: Refresh • ", key_hints::format_binding_opt(refresh)),
                Style::default().fg(Color::Green),
            ),
            Span::raw(format!(
                "{}: Settings • ",
                key_hints::format_binding_opt(settings)
            )),
            Span::raw(format!(
                "{}: Sort • ",
                key_hints::format_binding_opt(sort)
            )),
            Span::raw(format!("{}: Quit", key_hints::format_binding_opt(quit))),
        ])])
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(controls, area);
    }

    fn draw_search_bar(&self, f: &mut Frame, area: Rect) {
        let cfg = &self.key_config;
        let exit = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Search,
            InputAction::SearchExit,
            &[key_hints::kb(KeyCode::Esc)],
        );
        let play = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Search,
            InputAction::PlaySelected,
            &[key_hints::kb(KeyCode::Enter)],
        );
        let nav_up = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Search,
            InputAction::NavigateUp,
            &[key_hints::kb(KeyCode::Up)],
        );
        let nav_down = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Search,
            InputAction::NavigateDown,
            &[key_hints::kb(KeyCode::Down)],
        );
        let clear = key_hints::pick_binding_with_preference(
            cfg,
            InputMode::Search,
            InputAction::SearchClearLine,
            &[key_hints::kb_ctrl_char('u')],
        );

        let search_text = vec![
            Line::from(vec![
                Span::styled("Search: ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.search_query, Style::default().fg(Color::White)),
                Span::styled("█", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::raw(format!(
                    "{}: Exit • ",
                    key_hints::format_binding_opt(exit)
                )),
                Span::raw(format!(
                    "{}: Play • ",
                    key_hints::format_binding_opt(play)
                )),
                Span::raw(format!(
                    "{}/{}: Navigate • ",
                    key_hints::format_binding_opt(nav_up),
                    key_hints::format_binding_opt(nav_down)
                )),
                Span::raw("Backspace: Delete • "),
                Span::raw(format!(
                    "{}: Clear All",
                    key_hints::format_binding_opt(clear)
                )),
            ]),
        ];

        let paragraph = Paragraph::new(search_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title(" Search Mode "));
        f.render_widget(paragraph, area);
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

    fn poll_input(&mut self, config: &KeyConfig) -> Result<Vec<UiEvent>> {
        let mut events = Vec::new();

        if !self.key_config_synced {
            self.key_config = config.clone();
            self.key_config_synced = true;
        }

        if !event::poll(Duration::from_millis(0))? {
            return Ok(events);
        }

        let Event::Key(key) = event::read()? else {
            return Ok(events);
        };

        let mode = self.current_mode();

        if let Some(action) = map_key(mode, key, config) {
            self.apply_action(action, &mut events);
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
    fn current_mode(&self) -> InputMode {
        if self.settings.is_open() {
            if self.settings.is_editing_path() {
                InputMode::SettingsTextEntry
            } else {
                InputMode::Settings
            }
        } else if self.search_active {
            InputMode::Search
        } else {
            InputMode::Normal
        }
    }

    fn apply_action(&mut self, action: InputAction, events: &mut Vec<UiEvent>) {
        match action {
            InputAction::Quit => events.push(UiEvent::QuitRequested),
            InputAction::OpenSettings => self.settings.open(),

            InputAction::EnterSearch => {
                if !self.songs.is_empty() {
                    events.push(UiEvent::SearchToggled { active: true });
                }
            }
            InputAction::SearchExit => events.push(UiEvent::SearchToggled { active: false }),
            InputAction::SearchClearLine => {
                events.push(UiEvent::SearchQueryChanged {
                    query: String::new(),
                });
            }
            InputAction::SearchBackspace => {
                let mut q = self.search_query.clone();
                q.pop();
                events.push(UiEvent::SearchQueryChanged { query: q });
            }
            InputAction::SearchAppend(c) => {
                let mut q = self.search_query.clone();
                q.push(c);
                events.push(UiEvent::SearchQueryChanged { query: q });
            }

            InputAction::NavigateUp => {
                if let Some(index) = self.navigate_up() {
                    events.push(UiEvent::SelectionChanged { index });
                }
            }
            InputAction::NavigateDown => {
                if let Some(index) = self.navigate_down() {
                    events.push(UiEvent::SelectionChanged { index });
                }
            }

            InputAction::PlaySelected => events.push(UiEvent::PlaySelectedRequested),
            InputAction::TogglePause => events.push(UiEvent::TogglePauseRequested),
            InputAction::NextTrack => events.push(UiEvent::NextTrackRequested),
            InputAction::PreviousTrack => events.push(UiEvent::PreviousTrackRequested),
            InputAction::ToggleShuffle => events.push(UiEvent::ShuffleToggled {
                shuffle_enabled: self.shuffle,
            }),
            InputAction::Refresh => events.push(UiEvent::RefreshRequested),
            InputAction::CycleSort => events.push(UiEvent::SortCycleRequested),

            InputAction::SettingsClose
            | InputAction::SettingsNavigateUp
            | InputAction::SettingsNavigateDown
            | InputAction::SettingsConfirm
            | InputAction::SettingsLeft
            | InputAction::SettingsRight
            | InputAction::SettingsTypeChar(_)
            | InputAction::SettingsBackspace
            | InputAction::SettingsClearLine => events.extend(self.settings.apply_action(action)),
        }
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
