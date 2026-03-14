use crate::application::state::AppState;
use crate::core::events::UiEvent;
use crate::core::models::RepeatMode;
use crate::utils::{amplitude_to_volume, VOLUME_MAX, VOLUME_STEP};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const SETTINGS_FIELDS: &[SettingsField] = &[
    SettingsField::Volume,
    SettingsField::Repeat,
    SettingsField::MusicPath,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    MusicPath,
    Volume,
    Repeat,
}

/// Inline validation state for the path field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathValidation {
    /// User hasn't tried to confirm yet, or is still typing.
    Idle,
    /// Last confirm attempt failed
    Error(String),
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    open: bool,

    selected: SettingsField,

    editing_volume: bool,
    temp_volume: u8,

    temp_repeat: RepeatMode,

    editing_path: bool,
    temp_path: String,
    path_validation: PathValidation,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            open: false,
            selected: SettingsField::Volume,
            editing_volume: false,
            temp_volume: VOLUME_MAX,
            temp_repeat: RepeatMode::default(),
            editing_path: false,
            temp_path: String::new(),
            path_validation: PathValidation::Idle,
        }
    }
}

impl SettingsState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.editing_volume = false;
        self.editing_path = false;
        self.path_validation = PathValidation::Idle;
    }

    pub fn selected(&self) -> SettingsField {
        self.selected
    }

    pub fn is_editing_volume(&self) -> bool {
        self.editing_volume
    }

    pub fn is_editing_path(&self) -> bool {
        self.editing_path
    }

    pub fn temp_volume(&self) -> u8 {
        self.temp_volume
    }

    pub fn temp_repeat(&self) -> RepeatMode {
        self.temp_repeat
    }

    pub fn temp_path(&self) -> &str {
        &self.temp_path
    }

    pub fn path_validation(&self) -> &PathValidation {
        &self.path_validation
    }

    pub fn sync_from_app_state(&mut self, app_state: &AppState) {
        self.temp_repeat = app_state.config.repeat;

        if !self.editing_path {
            self.temp_path = app_state
                .config
                .root_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
        }

        if !self.editing_volume || self.selected != SettingsField::Volume {
            self.temp_volume = amplitude_to_volume(app_state.config.volume);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Vec<UiEvent> {
        let mut events = Vec::new();

        if self.editing_path {
            self.handle_path_editing_input(key, &mut events);
            return events;
        }

        if self.editing_volume {
            self.handle_volume_editing_input(key, &mut events);
            return events;
        }

        self.handle_navigation_input(key, &mut events);
        events
    }

    fn navigate_up(&mut self) {
        let current = SETTINGS_FIELDS
            .iter()
            .position(|f| *f == self.selected)
            .unwrap_or(0);
        let prev = (current + SETTINGS_FIELDS.len() - 1) % SETTINGS_FIELDS.len();
        self.selected = SETTINGS_FIELDS[prev];
    }

    fn navigate_down(&mut self) {
        let current = SETTINGS_FIELDS
            .iter()
            .position(|f| *f == self.selected)
            .unwrap_or(0);
        let next = (current + 1) % SETTINGS_FIELDS.len();
        self.selected = SETTINGS_FIELDS[next];
    }

    fn handle_volume_editing_input(&mut self, key: KeyEvent, events: &mut Vec<UiEvent>) {
        match key.code {
            KeyCode::Enter => {
                self.editing_volume = false;
                events.push(UiEvent::VolumeChangeRequested {
                    volume: self.temp_volume,
                });
            }
            KeyCode::Esc => {
                self.editing_volume = false;
            }
            KeyCode::Left => {
                self.temp_volume = self.temp_volume.saturating_sub(VOLUME_STEP);
            }
            KeyCode::Right => {
                self.temp_volume = self
                    .temp_volume
                    .saturating_add(VOLUME_STEP)
                    .min(VOLUME_MAX);
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let digit = c.to_digit(10).unwrap() as u8;
                let new_val = (self.temp_volume % 10) * 10 + digit;
                if new_val <= VOLUME_MAX {
                    self.temp_volume = new_val;
                }
            }
            _ => {}
        }
    }

    fn handle_path_editing_input(&mut self, key: KeyEvent, events: &mut Vec<UiEvent>) {
        match key.code {
            KeyCode::Enter => {
                let path = std::path::Path::new(&self.temp_path);
                if self.temp_path.is_empty() {
                    self.path_validation =
                        PathValidation::Error("Path cannot be empty.".to_string());
                } else if !path.exists() {
                    self.path_validation =
                        PathValidation::Error("Path does not exist.".to_string());
                } else if !path.is_dir() {
                    self.path_validation =
                        PathValidation::Error("Path is not a directory.".to_string());
                } else {
                    self.editing_path = false;
                    self.path_validation = PathValidation::Idle;
                    events.push(UiEvent::PathChangeRequested {
                        path: path.to_path_buf(),
                    });
                }
            }
            KeyCode::Esc => {
                self.editing_path = false;
                self.path_validation = PathValidation::Idle;
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.temp_path.clear();
                self.path_validation = PathValidation::Idle;
            }
            KeyCode::Backspace => {
                self.temp_path.pop();
                self.path_validation = PathValidation::Idle;
            }
            KeyCode::Char(c) => {
                self.temp_path.push(c);
                self.path_validation = PathValidation::Idle;
            }
            _ => {}
        }
    }

    fn handle_navigation_input(&mut self, key: KeyEvent, events: &mut Vec<UiEvent>) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Esc => {
                self.close();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_down();
            }
            KeyCode::Enter => match self.selected {
                SettingsField::Volume => {
                    self.editing_volume = true;
                }
                SettingsField::Repeat => {
                    self.temp_repeat = self.temp_repeat.cycle();
                    events.push(UiEvent::RepeatChangeRequested {
                        mode: self.temp_repeat,
                    });
                }
                SettingsField::MusicPath => {
                    self.editing_path = true;
                    self.path_validation = PathValidation::Idle;
                }
            },
            KeyCode::Left => {
                if self.selected == SettingsField::Repeat {
                    self.temp_repeat = self.temp_repeat.cycle_back();
                    events.push(UiEvent::RepeatChangeRequested {
                        mode: self.temp_repeat,
                    });
                }
            }
            KeyCode::Right => {
                if self.selected == SettingsField::Repeat {
                    self.temp_repeat = self.temp_repeat.cycle();
                    events.push(UiEvent::RepeatChangeRequested {
                        mode: self.temp_repeat,
                    });
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::UiEvent;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn volume_edit_confirm_emits_event() {
        let mut s = SettingsState::default();
        s.open();

        let events = s.handle_key(key(KeyCode::Enter));
        assert!(events.is_empty());
        assert!(s.is_editing_volume());

        s.handle_key(key(KeyCode::Right));
        let events = s.handle_key(key(KeyCode::Enter));

        assert!(!s.is_editing_volume());
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], UiEvent::VolumeChangeRequested { .. }));
    }

    #[test]
    fn path_edit_empty_sets_error() {
        let mut s = SettingsState::default();
        s.open();

        s.handle_key(key(KeyCode::Down));
        s.handle_key(key(KeyCode::Down));
        assert_eq!(s.selected(), SettingsField::MusicPath);

        s.handle_key(key(KeyCode::Enter));
        assert!(s.is_editing_path());

        let events = s.handle_key(key(KeyCode::Enter));
        assert!(events.is_empty());
        assert!(matches!(s.path_validation(), PathValidation::Error(_)));
        assert!(s.is_editing_path());
    }

    #[test]
    fn path_edit_valid_dir_emits_event_and_exits_edit() {
        let mut s = SettingsState::default();
        s.open();

        s.handle_key(key(KeyCode::Down));
        s.handle_key(key(KeyCode::Down));
        s.handle_key(key(KeyCode::Enter));

        let dir = std::env::temp_dir().join(format!(
            "music_cli_settings_state_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        for ch in dir.to_string_lossy().chars() {
            s.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }

        let events = s.handle_key(key(KeyCode::Enter));
        assert!(!s.is_editing_path());
        assert!(matches!(s.path_validation(), PathValidation::Idle));
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], UiEvent::PathChangeRequested { .. }));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ctrl_u_clears_path() {
        let mut s = SettingsState::default();
        s.open();

        s.handle_key(key(KeyCode::Down));
        s.handle_key(key(KeyCode::Down));
        s.handle_key(key(KeyCode::Enter));

        s.handle_key(key(KeyCode::Char('a')));
        assert_eq!(s.temp_path(), "a");

        s.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(s.temp_path(), "");
    }
}
