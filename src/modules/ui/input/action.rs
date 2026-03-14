#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    // Normal mode
    Quit,
    OpenSettings,
    EnterSearch,
    NavigateUp,
    NavigateDown,
    PlaySelected,
    TogglePause,
    NextTrack,
    PreviousTrack,
    ToggleShuffle,
    Refresh,
    CycleSort,

    // Search mode
    SearchExit,
    SearchClearLine,
    SearchBackspace,
    SearchAppend(char),

    // Settings mode
    SettingsClose,
    SettingsNavigateUp,
    SettingsNavigateDown,
    SettingsConfirm,
    SettingsLeft,
    SettingsRight,
    SettingsTypeChar(char),
    SettingsBackspace,
    SettingsClearLine,
}

