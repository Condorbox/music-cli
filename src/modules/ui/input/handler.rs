use crate::modules::ui::input::action::InputAction;
use crate::modules::ui::input::mode::InputMode;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn map(mode: InputMode, key: KeyEvent) -> Option<InputAction> {
    if matches!(key.kind, KeyEventKind::Release) {
        return None;
    }

    match mode {
        InputMode::Normal => map_normal(key),
        InputMode::Search => map_search(key),
        InputMode::Settings => map_settings(key),
        InputMode::SettingsTextEntry => map_settings_text_entry(key),
    }
}

fn map_normal(key: KeyEvent) -> Option<InputAction> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Some(InputAction::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::Quit)
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            if matches!(key.kind, KeyEventKind::Repeat) {
                None
            } else {
                Some(InputAction::OpenSettings)
            }
        }
        KeyCode::Char('/') => Some(InputAction::EnterSearch),
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::EnterSearch)
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Some(InputAction::NavigateUp),
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Some(InputAction::NavigateDown),
        KeyCode::Enter => Some(InputAction::PlaySelected),
        KeyCode::Char(' ') | KeyCode::Char('p') | KeyCode::Char('P') => Some(InputAction::TogglePause),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Right => Some(InputAction::NextTrack),
        KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Left => Some(InputAction::PreviousTrack),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(InputAction::ToggleShuffle),
        KeyCode::F(5) | KeyCode::Char('u') | KeyCode::Char('U') => Some(InputAction::Refresh),
        KeyCode::Char('o') | KeyCode::Char('O') => Some(InputAction::CycleSort),
        _ => None,
    }
}

fn map_search(key: KeyEvent) -> Option<InputAction> {
    match key.code {
        KeyCode::Esc => Some(InputAction::SearchExit),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::SearchClearLine)
        }
        KeyCode::Backspace => Some(InputAction::SearchBackspace),
        KeyCode::Up => Some(InputAction::NavigateUp),
        KeyCode::Down => Some(InputAction::NavigateDown),
        KeyCode::Enter => Some(InputAction::PlaySelected),
        KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::TogglePause)
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::SearchAppend(c))
        }
        _ => None,
    }
}

fn map_settings(key: KeyEvent) -> Option<InputAction> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('s') | KeyCode::Char('S') => {
            if matches!(key.kind, KeyEventKind::Repeat) {
                None
            } else {
                Some(InputAction::SettingsClose)
            }
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Some(InputAction::SettingsNavigateUp),
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            Some(InputAction::SettingsNavigateDown)
        }
        KeyCode::Enter => Some(InputAction::SettingsConfirm),
        KeyCode::Left => Some(InputAction::SettingsLeft),
        KeyCode::Right => Some(InputAction::SettingsRight),
        KeyCode::Backspace => Some(InputAction::SettingsBackspace),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::SettingsClearLine)
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::SettingsTypeChar(c))
        }
        _ => None,
    }
}

fn map_settings_text_entry(key: KeyEvent) -> Option<InputAction> {
    match key.code {
        KeyCode::Esc => {
            if matches!(key.kind, KeyEventKind::Repeat) {
                None
            } else {
                Some(InputAction::SettingsClose)
            }
        }
        KeyCode::Enter => Some(InputAction::SettingsConfirm),
        KeyCode::Backspace => Some(InputAction::SettingsBackspace),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::SettingsClearLine)
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::SettingsTypeChar(c))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn key_release_is_ignored() {
        let release = KeyEvent::new_with_kind(KeyCode::Char('s'), KeyModifiers::NONE, KeyEventKind::Release);
        assert_eq!(map(InputMode::Normal, release), None);
        assert_eq!(map(InputMode::Settings, release), None);
        assert_eq!(map(InputMode::SettingsTextEntry, release), None);
    }

    #[test]
    fn repeat_s_does_not_toggle_settings() {
        let repeat = KeyEvent::new_with_kind(KeyCode::Char('s'), KeyModifiers::NONE, KeyEventKind::Repeat);
        assert_eq!(map(InputMode::Normal, repeat), None);
        assert_eq!(map(InputMode::Settings, repeat), None);
    }

    #[test]
    fn normal_mode_bindings() {
        assert_eq!(map(InputMode::Normal, key(KeyCode::Char('q'))), Some(InputAction::Quit));
        assert_eq!(map(InputMode::Normal, key(KeyCode::Char('Q'))), Some(InputAction::Quit));
        assert_eq!(map(InputMode::Normal, key(KeyCode::Esc)), Some(InputAction::Quit));
        assert_eq!(map(InputMode::Normal, ctrl(KeyCode::Char('c'))), Some(InputAction::Quit));

        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('s'))),
            Some(InputAction::OpenSettings)
        );

        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('/'))),
            Some(InputAction::EnterSearch)
        );
        assert_eq!(
            map(InputMode::Normal, ctrl(KeyCode::Char('f'))),
            Some(InputAction::EnterSearch)
        );

        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Up)),
            Some(InputAction::NavigateUp)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Down)),
            Some(InputAction::NavigateDown)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Enter)),
            Some(InputAction::PlaySelected)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char(' '))),
            Some(InputAction::TogglePause)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Right)),
            Some(InputAction::NextTrack)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Left)),
            Some(InputAction::PreviousTrack)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('r'))),
            Some(InputAction::ToggleShuffle)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::F(5))),
            Some(InputAction::Refresh)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('o'))),
            Some(InputAction::CycleSort)
        );
    }

    #[test]
    fn search_mode_bindings() {
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Esc)),
            Some(InputAction::SearchExit)
        );
        assert_eq!(
            map(InputMode::Search, ctrl(KeyCode::Char('u'))),
            Some(InputAction::SearchClearLine)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Backspace)),
            Some(InputAction::SearchBackspace)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Up)),
            Some(InputAction::NavigateUp)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Down)),
            Some(InputAction::NavigateDown)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Enter)),
            Some(InputAction::PlaySelected)
        );
        assert_eq!(
            map(InputMode::Search, ctrl(KeyCode::Char(' '))),
            Some(InputAction::TogglePause)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Char('a'))),
            Some(InputAction::SearchAppend('a'))
        );
    }

    #[test]
    fn settings_mode_bindings() {
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Esc)),
            Some(InputAction::SettingsClose)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Up)),
            Some(InputAction::SettingsNavigateUp)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Down)),
            Some(InputAction::SettingsNavigateDown)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Enter)),
            Some(InputAction::SettingsConfirm)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Left)),
            Some(InputAction::SettingsLeft)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Right)),
            Some(InputAction::SettingsRight)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Backspace)),
            Some(InputAction::SettingsBackspace)
        );
        assert_eq!(
            map(InputMode::Settings, ctrl(KeyCode::Char('u'))),
            Some(InputAction::SettingsClearLine)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Char('a'))),
            Some(InputAction::SettingsTypeChar('a'))
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Char('s'))),
            Some(InputAction::SettingsClose)
        );
    }

    #[test]
    fn settings_text_entry_maps_all_chars_including_close_toggle_key() {
        assert_eq!(
            map(InputMode::SettingsTextEntry, key(KeyCode::Esc)),
            Some(InputAction::SettingsClose)
        );
        assert_eq!(
            map(InputMode::SettingsTextEntry, key(KeyCode::Enter)),
            Some(InputAction::SettingsConfirm)
        );
        assert_eq!(
            map(InputMode::SettingsTextEntry, key(KeyCode::Backspace)),
            Some(InputAction::SettingsBackspace)
        );
        assert_eq!(
            map(InputMode::SettingsTextEntry, ctrl(KeyCode::Char('u'))),
            Some(InputAction::SettingsClearLine)
        );
        assert_eq!(
            map(InputMode::SettingsTextEntry, key(KeyCode::Char('a'))),
            Some(InputAction::SettingsTypeChar('a'))
        );
        assert_eq!(
            map(InputMode::SettingsTextEntry, key(KeyCode::Char('s'))),
            Some(InputAction::SettingsTypeChar('s'))
        );

        let repeat_s = KeyEvent::new_with_kind(KeyCode::Char('s'), KeyModifiers::NONE, KeyEventKind::Repeat);
        assert_eq!(
            map(InputMode::SettingsTextEntry, repeat_s),
            Some(InputAction::SettingsTypeChar('s'))
        );
    }

    #[test]
    fn mode_isolation_examples() {
        assert_eq!(map(InputMode::Normal, key(KeyCode::Esc)), Some(InputAction::Quit));
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Esc)),
            Some(InputAction::SearchExit)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Esc)),
            Some(InputAction::SettingsClose)
        );

        assert_eq!(map(InputMode::Normal, key(KeyCode::Left)), Some(InputAction::PreviousTrack));
        assert_eq!(map(InputMode::Search, key(KeyCode::Left)), None);
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Left)),
            Some(InputAction::SettingsLeft)
        );

        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('j'))),
            Some(InputAction::NavigateDown)
        );
        assert_eq!(map(InputMode::Search, key(KeyCode::Char('j'))), Some(InputAction::SearchAppend('j')));
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Char('j'))),
            Some(InputAction::SettingsNavigateDown)
        );

        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('k'))),
            Some(InputAction::NavigateUp)
        );
        assert_eq!(map(InputMode::Search, key(KeyCode::Char('k'))), Some(InputAction::SearchAppend('k')));
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Char('k'))),
            Some(InputAction::SettingsNavigateUp)
        );
    }
}
