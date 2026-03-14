use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputContext {
    pub has_songs: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Quit,
    OpenSettings,

    SearchEnter,
    SearchExit,
    SearchClearAll,
    SearchBackspace,
    SearchAppend(char),

    NavigateUp,
    NavigateDown,

    PlaySelected,
    TogglePause,
    NextTrack,
    PreviousTrack,
    ShuffleToggle,
    Refresh,
    SortCycle,
}

#[derive(Default)]
pub struct InputHandler;

impl InputHandler {
    pub fn handle_key(
        &self,
        mode: InputMode,
        ctx: InputContext,
        key: KeyEvent,
    ) -> Option<InputAction> {
        match mode {
            InputMode::Normal => self.handle_normal(ctx, key),
            InputMode::Search => self.handle_search(key),
        }
    }

    fn handle_normal(&self, ctx: InputContext, key: KeyEvent) -> Option<InputAction> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(InputAction::Quit),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(InputAction::Quit)
            }
            KeyCode::Char('s') => Some(InputAction::OpenSettings),
            KeyCode::Char('/') if ctx.has_songs => Some(InputAction::SearchEnter),
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) && ctx.has_songs => {
                Some(InputAction::SearchEnter)
            }
            KeyCode::Up | KeyCode::Char('k') => Some(InputAction::NavigateUp),
            KeyCode::Down | KeyCode::Char('j') => Some(InputAction::NavigateDown),
            KeyCode::Enter => Some(InputAction::PlaySelected),
            KeyCode::Char(' ') | KeyCode::Char('p') => Some(InputAction::TogglePause),
            KeyCode::Char('n') | KeyCode::Right => Some(InputAction::NextTrack),
            KeyCode::Char('b') | KeyCode::Left => Some(InputAction::PreviousTrack),
            KeyCode::Char('r') => Some(InputAction::ShuffleToggle),
            KeyCode::F(5) | KeyCode::Char('u') => Some(InputAction::Refresh),
            KeyCode::Char('o') => Some(InputAction::SortCycle),
            _ => None,
        }
    }

    fn handle_search(&self, key: KeyEvent) -> Option<InputAction> {
        match key.code {
            KeyCode::Esc => Some(InputAction::SearchExit),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(InputAction::SearchClearAll)
            }
            KeyCode::Backspace => Some(InputAction::SearchBackspace),
            KeyCode::Up => Some(InputAction::NavigateUp),
            KeyCode::Down => Some(InputAction::NavigateDown),
            KeyCode::Enter => Some(InputAction::PlaySelected),
            KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(InputAction::TogglePause)
            }
            KeyCode::Char(c) => Some(InputAction::SearchAppend(c)),
            _ => None,
        }
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
    fn normal_gates_search_on_has_songs() {
        let h = InputHandler::default();
        let ctx = InputContext { has_songs: false };
        assert_eq!(h.handle_key(InputMode::Normal, ctx, key(KeyCode::Char('/'))), None);
        assert_eq!(h.handle_key(InputMode::Normal, ctx, ctrl(KeyCode::Char('f'))), None);

        let ctx = InputContext { has_songs: true };
        assert_eq!(
            h.handle_key(InputMode::Normal, ctx, key(KeyCode::Char('/'))),
            Some(InputAction::SearchEnter)
        );
        assert_eq!(
            h.handle_key(InputMode::Normal, ctx, ctrl(KeyCode::Char('f'))),
            Some(InputAction::SearchEnter)
        );
    }

    #[test]
    fn normal_basic_mappings() {
        let h = InputHandler::default();
        let ctx = InputContext { has_songs: true };

        assert_eq!(
            h.handle_key(InputMode::Normal, ctx, key(KeyCode::Char('q'))),
            Some(InputAction::Quit)
        );
        assert_eq!(
            h.handle_key(InputMode::Normal, ctx, key(KeyCode::Esc)),
            Some(InputAction::Quit)
        );
        assert_eq!(
            h.handle_key(InputMode::Normal, ctx, ctrl(KeyCode::Char('c'))),
            Some(InputAction::Quit)
        );
        assert_eq!(
            h.handle_key(InputMode::Normal, ctx, key(KeyCode::Char('s'))),
            Some(InputAction::OpenSettings)
        );
        assert_eq!(
            h.handle_key(InputMode::Normal, ctx, key(KeyCode::Up)),
            Some(InputAction::NavigateUp)
        );
        assert_eq!(
            h.handle_key(InputMode::Normal, ctx, key(KeyCode::Down)),
            Some(InputAction::NavigateDown)
        );
    }

    #[test]
    fn search_mappings() {
        let h = InputHandler::default();
        let ctx = InputContext { has_songs: true };

        assert_eq!(
            h.handle_key(InputMode::Search, ctx, key(KeyCode::Esc)),
            Some(InputAction::SearchExit)
        );
        assert_eq!(
            h.handle_key(InputMode::Search, ctx, ctrl(KeyCode::Char('u'))),
            Some(InputAction::SearchClearAll)
        );
        assert_eq!(
            h.handle_key(InputMode::Search, ctx, key(KeyCode::Backspace)),
            Some(InputAction::SearchBackspace)
        );
        assert_eq!(
            h.handle_key(InputMode::Search, ctx, key(KeyCode::Char('a'))),
            Some(InputAction::SearchAppend('a'))
        );
        assert_eq!(
            h.handle_key(InputMode::Search, ctx, ctrl(KeyCode::Char(' '))),
            Some(InputAction::TogglePause)
        );
    }
}

