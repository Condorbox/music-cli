#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum InputMode {
    #[default]
    Normal,
    Search,
    Settings,
    /// Settings modal is open and currently accepting free-form text input.
    ///
    /// In this mode, character keys should insert text instead of triggering
    /// modal-level shortcuts (e.g. the "close settings" toggle key).
    SettingsTextEntry,
}
