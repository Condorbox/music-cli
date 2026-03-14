use super::{InputAction, InputMode, KeyBinding, KeyConfig};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn map(mode: InputMode, key: KeyEvent, config: &KeyConfig) -> Option<InputAction> {
    if matches!(key.kind, KeyEventKind::Release) {
        return None;
    }

    let binding = KeyBinding::from_event(key);

    if matches!(key.kind, KeyEventKind::Repeat) && config.is_repeat_suppressed(mode, &binding) {
        return None;
    }

    match mode {
        InputMode::Search => {
            if let Some(action) = handle_search_text_input(key) {
                return Some(action);
            }
        }
        InputMode::SettingsTextEntry => {
            return handle_settings_text_entry(key);
        }
        InputMode::Settings => {
            if matches!(key.code, KeyCode::Backspace) {
                return Some(InputAction::SettingsBackspace);
            }
        }
        _ => {}
    }

    if let Some(action) = config.get(mode, &binding) {
        return Some(action);
    }

    match mode {
        InputMode::Settings => handle_settings_fallback_text_input(key),
        _ => None,
    }
}

fn handle_search_text_input(key: KeyEvent) -> Option<InputAction> {
    match key.code {
        KeyCode::Backspace => Some(InputAction::SearchBackspace),
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::SearchAppend(c))
        }
        _ => None,
    }
}

fn handle_settings_text_entry(key: KeyEvent) -> Option<InputAction> {
    match key.code {
        KeyCode::Esc => Some(InputAction::SettingsClose),
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

fn handle_settings_fallback_text_input(key: KeyEvent) -> Option<InputAction> {
    match key.code {
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputAction::SettingsTypeChar(c))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::input::key_config::KeyConfig;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn key_release_is_ignored() {
        let cfg = KeyConfig::default();
        let release = KeyEvent::new_with_kind(KeyCode::Char('s'), KeyModifiers::NONE, KeyEventKind::Release);
        assert_eq!(map(InputMode::Normal, release, &cfg), None);
        assert_eq!(map(InputMode::Settings, release, &cfg), None);
        assert_eq!(map(InputMode::SettingsTextEntry, release, &cfg), None);
    }

    #[test]
    fn repeat_s_does_not_toggle_settings() {
        let cfg = KeyConfig::default();
        let repeat = KeyEvent::new_with_kind(KeyCode::Char('s'), KeyModifiers::NONE, KeyEventKind::Repeat);
        assert_eq!(map(InputMode::Normal, repeat, &cfg), None);
        assert_eq!(map(InputMode::Settings, repeat, &cfg), None);
    }

    #[test]
    fn normal_mode_bindings() {
        let cfg = KeyConfig::default();
        assert_eq!(map(InputMode::Normal, key(KeyCode::Char('q')), &cfg), Some(InputAction::Quit));
        assert_eq!(map(InputMode::Normal, key(KeyCode::Char('Q')), &cfg), Some(InputAction::Quit));
        assert_eq!(map(InputMode::Normal, key(KeyCode::Esc), &cfg), Some(InputAction::Quit));
        assert_eq!(map(InputMode::Normal, ctrl(KeyCode::Char('c')), &cfg), Some(InputAction::Quit));

        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('s')), &cfg),
            Some(InputAction::OpenSettings)
        );

        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('/')), &cfg),
            Some(InputAction::EnterSearch)
        );
        assert_eq!(
            map(InputMode::Normal, ctrl(KeyCode::Char('f')), &cfg),
            Some(InputAction::EnterSearch)
        );

        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Up), &cfg),
            Some(InputAction::NavigateUp)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Down), &cfg),
            Some(InputAction::NavigateDown)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Enter), &cfg),
            Some(InputAction::PlaySelected)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char(' ')), &cfg),
            Some(InputAction::TogglePause)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Right), &cfg),
            Some(InputAction::NextTrack)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Left), &cfg),
            Some(InputAction::PreviousTrack)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('r')), &cfg),
            Some(InputAction::ToggleShuffle)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::F(5)), &cfg),
            Some(InputAction::Refresh)
        );
        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('o')), &cfg),
            Some(InputAction::CycleSort)
        );
    }

    #[test]
    fn search_mode_bindings() {
        let cfg = KeyConfig::default();
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Esc), &cfg),
            Some(InputAction::SearchExit)
        );
        assert_eq!(
            map(InputMode::Search, ctrl(KeyCode::Char('u')), &cfg),
            Some(InputAction::SearchClearLine)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Backspace), &cfg),
            Some(InputAction::SearchBackspace)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Up), &cfg),
            Some(InputAction::NavigateUp)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Down), &cfg),
            Some(InputAction::NavigateDown)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Enter), &cfg),
            Some(InputAction::PlaySelected)
        );
        assert_eq!(
            map(InputMode::Search, ctrl(KeyCode::Char(' ')), &cfg),
            Some(InputAction::TogglePause)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Char('a')), &cfg),
            Some(InputAction::SearchAppend('a'))
        );
    }

    #[test]
    fn settings_mode_bindings() {
        let cfg = KeyConfig::default();
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Esc), &cfg),
            Some(InputAction::SettingsClose)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Up), &cfg),
            Some(InputAction::SettingsNavigateUp)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Down), &cfg),
            Some(InputAction::SettingsNavigateDown)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Enter), &cfg),
            Some(InputAction::SettingsConfirm)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Left), &cfg),
            Some(InputAction::SettingsLeft)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Right), &cfg),
            Some(InputAction::SettingsRight)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Backspace), &cfg),
            Some(InputAction::SettingsBackspace)
        );
        assert_eq!(
            map(InputMode::Settings, ctrl(KeyCode::Char('u')), &cfg),
            Some(InputAction::SettingsClearLine)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Char('a')), &cfg),
            Some(InputAction::SettingsTypeChar('a'))
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Char('s')), &cfg),
            Some(InputAction::SettingsClose)
        );
    }

    #[test]
    fn settings_text_entry_maps_all_chars_including_close_toggle_key() {
        let cfg = KeyConfig::default();
        assert_eq!(
            map(InputMode::SettingsTextEntry, key(KeyCode::Esc), &cfg),
            Some(InputAction::SettingsClose)
        );
        assert_eq!(
            map(InputMode::SettingsTextEntry, key(KeyCode::Enter), &cfg),
            Some(InputAction::SettingsConfirm)
        );
        assert_eq!(
            map(InputMode::SettingsTextEntry, key(KeyCode::Backspace), &cfg),
            Some(InputAction::SettingsBackspace)
        );
        assert_eq!(
            map(InputMode::SettingsTextEntry, ctrl(KeyCode::Char('u')), &cfg),
            Some(InputAction::SettingsClearLine)
        );
        assert_eq!(
            map(InputMode::SettingsTextEntry, key(KeyCode::Char('a')), &cfg),
            Some(InputAction::SettingsTypeChar('a'))
        );
        assert_eq!(
            map(InputMode::SettingsTextEntry, key(KeyCode::Char('s')), &cfg),
            Some(InputAction::SettingsTypeChar('s'))
        );

        let repeat_s = KeyEvent::new_with_kind(KeyCode::Char('s'), KeyModifiers::NONE, KeyEventKind::Repeat);
        assert_eq!(
            map(InputMode::SettingsTextEntry, repeat_s, &cfg),
            Some(InputAction::SettingsTypeChar('s'))
        );
    }

    #[test]
    fn mode_isolation_examples() {
        let cfg = KeyConfig::default();
        assert_eq!(map(InputMode::Normal, key(KeyCode::Esc), &cfg), Some(InputAction::Quit));
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Esc), &cfg),
            Some(InputAction::SearchExit)
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Esc), &cfg),
            Some(InputAction::SettingsClose)
        );

        assert_eq!(map(InputMode::Normal, key(KeyCode::Left), &cfg), Some(InputAction::PreviousTrack));
        assert_eq!(map(InputMode::Search, key(KeyCode::Left), &cfg), None);
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Left), &cfg),
            Some(InputAction::SettingsLeft)
        );

        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('j')), &cfg),
            Some(InputAction::NavigateDown)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Char('j')), &cfg),
            Some(InputAction::SearchAppend('j'))
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Char('j')), &cfg),
            Some(InputAction::SettingsNavigateDown)
        );

        assert_eq!(
            map(InputMode::Normal, key(KeyCode::Char('k')), &cfg),
            Some(InputAction::NavigateUp)
        );
        assert_eq!(
            map(InputMode::Search, key(KeyCode::Char('k')), &cfg),
            Some(InputAction::SearchAppend('k'))
        );
        assert_eq!(
            map(InputMode::Settings, key(KeyCode::Char('k')), &cfg),
            Some(InputAction::SettingsNavigateUp)
        );
    }
}
