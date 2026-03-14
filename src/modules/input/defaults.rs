use super::{InputAction, InputMode, KeyBinding};
use crossterm::event::{KeyCode, KeyModifiers};
use std::collections::HashSet;

/// Returns all compiled-in default bindings as a flat list.
///
/// Each entry is (mode, binding, action).
/// Multi-key bindings (e.g. both 'q' and 'Esc' → Quit) appear as separate entries.
/// Text-input actions (SearchAppend, SettingsTypeChar, *Backspace) are NOT included
/// those are handled structurally in handler.rs, not via the config map.
pub fn default_bindings() -> Vec<(InputMode, KeyBinding, InputAction)> {
    let mut bindings = Vec::new();

    // Normal mode
    push_normal(&mut bindings, "q", InputAction::Quit);
    push_normal_special(&mut bindings, KeyCode::Esc, KeyModifiers::NONE, InputAction::Quit);
    push_normal_special(
        &mut bindings,
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
        InputAction::Quit,
    );

    push_normal(&mut bindings, "s", InputAction::OpenSettings);

    push_normal(&mut bindings, "/", InputAction::EnterSearch);
    push_normal_special(
        &mut bindings,
        KeyCode::Char('f'),
        KeyModifiers::CONTROL,
        InputAction::EnterSearch,
    );

    push_normal_special(&mut bindings, KeyCode::Up, KeyModifiers::NONE, InputAction::NavigateUp);
    push_normal(&mut bindings, "k", InputAction::NavigateUp);

    push_normal_special(
        &mut bindings,
        KeyCode::Down,
        KeyModifiers::NONE,
        InputAction::NavigateDown,
    );
    push_normal(&mut bindings, "j", InputAction::NavigateDown);

    push_normal_special(
        &mut bindings,
        KeyCode::Enter,
        KeyModifiers::NONE,
        InputAction::PlaySelected,
    );

    push_normal_special(
        &mut bindings,
        KeyCode::Char(' '),
        KeyModifiers::NONE,
        InputAction::TogglePause,
    );
    push_normal(&mut bindings, "p", InputAction::TogglePause);

    push_normal(&mut bindings, "n", InputAction::NextTrack);
    push_normal_special(
        &mut bindings,
        KeyCode::Right,
        KeyModifiers::NONE,
        InputAction::NextTrack,
    );

    push_normal(&mut bindings, "b", InputAction::PreviousTrack);
    push_normal_special(
        &mut bindings,
        KeyCode::Left,
        KeyModifiers::NONE,
        InputAction::PreviousTrack,
    );

    push_normal(&mut bindings, "r", InputAction::ToggleShuffle);

    push_normal_special(
        &mut bindings,
        KeyCode::F(5),
        KeyModifiers::NONE,
        InputAction::Refresh,
    );
    push_normal(&mut bindings, "u", InputAction::Refresh);

    push_normal(&mut bindings, "o", InputAction::CycleSort);

    // Search mode (text input actions are structural and intentionally omitted)
    bindings.push((
        InputMode::Search,
        KeyBinding::from_str("Esc").expect("Esc must parse"),
        InputAction::SearchExit,
    ));
    bindings.push((
        InputMode::Search,
        KeyBinding::from_str("Ctrl+u").expect("Ctrl+u must parse"),
        InputAction::SearchClearLine,
    ));
    bindings.push((
        InputMode::Search,
        KeyBinding::from_str("Up").expect("Up must parse"),
        InputAction::NavigateUp,
    ));
    bindings.push((
        InputMode::Search,
        KeyBinding::from_str("Down").expect("Down must parse"),
        InputAction::NavigateDown,
    ));
    bindings.push((
        InputMode::Search,
        KeyBinding::from_str("Enter").expect("Enter must parse"),
        InputAction::PlaySelected,
    ));
    bindings.push((
        InputMode::Search,
        KeyBinding::from_str("Ctrl+Space").expect("Ctrl+Space must parse"),
        InputAction::TogglePause,
    ));

    // Settings mode (structural text input actions are intentionally omitted)
    bindings.push((
        InputMode::Settings,
        KeyBinding::from_str("Esc").expect("Esc must parse"),
        InputAction::SettingsClose,
    ));
    bindings.push((
        InputMode::Settings,
        KeyBinding::from_str("s").expect("s must parse"),
        InputAction::SettingsClose,
    ));
    bindings.push((
        InputMode::Settings,
        KeyBinding::from_str("Up").expect("Up must parse"),
        InputAction::SettingsNavigateUp,
    ));
    bindings.push((
        InputMode::Settings,
        KeyBinding::from_str("k").expect("k must parse"),
        InputAction::SettingsNavigateUp,
    ));
    bindings.push((
        InputMode::Settings,
        KeyBinding::from_str("Down").expect("Down must parse"),
        InputAction::SettingsNavigateDown,
    ));
    bindings.push((
        InputMode::Settings,
        KeyBinding::from_str("j").expect("j must parse"),
        InputAction::SettingsNavigateDown,
    ));
    bindings.push((
        InputMode::Settings,
        KeyBinding::from_str("Enter").expect("Enter must parse"),
        InputAction::SettingsConfirm,
    ));
    bindings.push((
        InputMode::Settings,
        KeyBinding::from_str("Left").expect("Left must parse"),
        InputAction::SettingsLeft,
    ));
    bindings.push((
        InputMode::Settings,
        KeyBinding::from_str("Right").expect("Right must parse"),
        InputAction::SettingsRight,
    ));
    bindings.push((
        InputMode::Settings,
        KeyBinding::from_str("Ctrl+u").expect("Ctrl+u must parse"),
        InputAction::SettingsClearLine,
    ));

    // SettingsTextEntry mode (not exposed in config; included for repeat suppression + invariants)
    bindings.push((
        InputMode::SettingsTextEntry,
        KeyBinding::from_str("Esc").expect("Esc must parse"),
        InputAction::SettingsClose,
    ));
    bindings.push((
        InputMode::SettingsTextEntry,
        KeyBinding::from_str("Enter").expect("Enter must parse"),
        InputAction::SettingsConfirm,
    ));
    bindings.push((
        InputMode::SettingsTextEntry,
        KeyBinding::from_str("Ctrl+u").expect("Ctrl+u must parse"),
        InputAction::SettingsClearLine,
    ));

    debug_assert_no_duplicates(&bindings);
    bindings
}

fn push_normal(out: &mut Vec<(InputMode, KeyBinding, InputAction)>, key: &str, action: InputAction) {
    out.push((
        InputMode::Normal,
        KeyBinding::from_str(key).unwrap_or_else(|e| panic!("Invalid default binding '{key}': {e}")),
        action,
    ));
}

fn push_normal_special(
    out: &mut Vec<(InputMode, KeyBinding, InputAction)>,
    code: KeyCode,
    modifiers: KeyModifiers,
    action: InputAction,
) {
    out.push((
        InputMode::Normal,
        KeyBinding { code, modifiers },
        action,
    ));
}

fn debug_assert_no_duplicates(bindings: &[(InputMode, KeyBinding, InputAction)]) {
    if !cfg!(debug_assertions) {
        return;
    }

    let mut seen = HashSet::new();
    for (mode, binding, _action) in bindings {
        assert!(
            seen.insert((*mode, *binding)),
            "Duplicate default binding for mode={mode:?}, key={binding}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn contains_all_bindable_actions() {
        let bindings = default_bindings();

        let mut actions: HashSet<InputAction> =
            bindings.iter().map(|(_, _, a)| *a).collect();

        // All actions except structural text-input ones.
        let expected = [
            InputAction::Quit,
            InputAction::OpenSettings,
            InputAction::EnterSearch,
            InputAction::NavigateUp,
            InputAction::NavigateDown,
            InputAction::PlaySelected,
            InputAction::TogglePause,
            InputAction::NextTrack,
            InputAction::PreviousTrack,
            InputAction::ToggleShuffle,
            InputAction::Refresh,
            InputAction::CycleSort,
            InputAction::SearchExit,
            InputAction::SearchClearLine,
            InputAction::SettingsClose,
            InputAction::SettingsNavigateUp,
            InputAction::SettingsNavigateDown,
            InputAction::SettingsConfirm,
            InputAction::SettingsLeft,
            InputAction::SettingsRight,
            InputAction::SettingsClearLine,
        ];

        for action in expected {
            assert!(actions.remove(&action), "Missing default binding for {action:?}");
        }
    }

    #[test]
    fn no_duplicate_mode_binding_pairs() {
        let bindings = default_bindings();
        let mut seen = HashSet::new();

        for (mode, binding, _action) in bindings {
            assert!(
                seen.insert((mode, binding)),
                "Duplicate binding: mode={mode:?}, key={binding}"
            );
        }
    }
}
