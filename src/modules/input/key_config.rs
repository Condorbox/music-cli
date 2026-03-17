use super::{defaults, InputAction, InputMode, KeyBinding};
use crossterm::event::{KeyCode, KeyModifiers};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use crate::utils::APP_NAME;

/// Resolved, lookup-ready keymap.
///
/// Constructed once at startup and treated as immutable thereafter.
/// Passed by shared reference into poll_input on every event loop tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyConfig {
    bindings: HashMap<(InputMode, KeyBinding), InputAction>,
    repeat_suppressed: HashSet<(InputMode, KeyBinding)>,
}

impl KeyConfig {
    pub fn default() -> Self {
        let mut bindings = HashMap::new();
        for (mode, binding, action) in defaults::default_bindings() {
            bindings.insert((mode, binding), action);
        }

        let repeat_suppressed = build_repeat_suppressed(&bindings);

        Self {
            bindings,
            repeat_suppressed,
        }
    }

    pub fn load_or_default(config_dir: &Path) -> Self {
        let file_path = keymap_path(config_dir);

        match fs::read_to_string(&file_path) {
            Ok(toml_str) => match Self::parse(&toml_str) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!(
                        "Warning: Failed to parse keymap at '{}': {err}. Using defaults.",
                        file_path.display()
                    );
                    Self::default()
                }
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                let default = Self::default();
                if let Err(write_err) = write_default_keymap_if_missing(&file_path) {
                    eprintln!(
                        "Warning: Could not create default keymap at '{}': {write_err}",
                        file_path.display()
                    );
                }
                default
            }
            Err(err) => {
                eprintln!(
                    "Warning: Could not read keymap at '{}': {err}. Using defaults.",
                    file_path.display()
                );
                Self::default()
            }
        }
    }

    pub fn get(&self, mode: InputMode, binding: &KeyBinding) -> Option<InputAction> {
        self.bindings.get(&(mode, *binding)).copied()
    }

    pub fn is_repeat_suppressed(&self, mode: InputMode, binding: &KeyBinding) -> bool {
        self.repeat_suppressed.contains(&(mode, *binding))
    }

    /// Returns all bindings for the given action in the given mode.
    ///
    /// The result is sorted by the binding's display string for stable UI output.
    pub fn bindings_for_action(&self, mode: InputMode, action: InputAction) -> Vec<KeyBinding> {
        let mut out: Vec<KeyBinding> = self
            .bindings
            .iter()
            .filter_map(|((m, binding), a)| {
                if *m == mode && *a == action {
                    Some(*binding)
                } else {
                    None
                }
            })
            .collect();

        out.sort_by_cached_key(|a| a.to_string());
        out.dedup();
        out
    }

    /// Returns bindings that are meaningful to display for the given mode.
    ///
    /// Some keys are structurally reserved by the input handler and will never reach
    /// the config lookup (e.g. typing in search). Those are filtered out to avoid
    /// showing misleading hints in the UI.
    pub fn hint_bindings_for_action(&self, mode: InputMode, action: InputAction) -> Vec<KeyBinding> {
        let mut out = self.bindings_for_action(mode, action);
        out.retain(|binding| is_hint_reachable_binding(mode, binding));
        out
    }

    fn parse(toml_str: &str) -> Result<Self, String> {
        let value: toml::Value =
            toml::from_str(toml_str).map_err(|e| format!("TOML error: {e}"))?;

        let mut config = Self::default();

        let Some(root) = value.as_table() else {
            return Err("Expected a TOML table at the root".to_string());
        };

        for (section_name, section_value) in root {
            let (mode, section_kind) = match section_name.as_str() {
                "normal" => (InputMode::Normal, SectionKind::Normal),
                "search" => (InputMode::Search, SectionKind::Search),
                "settings" => (InputMode::Settings, SectionKind::Settings),
                // SettingsTextEntry is intentionally not supported.
                other => {
                    eprintln!("Warning: Unknown keymap section '[{other}]' ignored.");
                    continue;
                }
            };

            let Some(table) = section_value.as_table() else {
                eprintln!(
                    "Warning: keymap section '[{section_name}]' must be a table; ignoring."
                );
                continue;
            };

            for (key_name, key_value) in table {
                let Some(action) = action_for_key(section_kind, key_name) else {
                    eprintln!(
                        "Warning: Unknown key '{key_name}' in section '[{section_name}]' ignored."
                    );
                    continue;
                };

                let key_strings = match parse_key_list(key_value) {
                    Ok(v) => v,
                    Err(err) => {
                        eprintln!(
                            "Warning: Invalid value for '{section_name}.{key_name}': {err}. Ignoring."
                        );
                        continue;
                    }
                };

                config.clear_action(mode, action);

                for key_str in key_strings {
                    let binding = match KeyBinding::from_str(&key_str) {
                        Ok(b) => b,
                        Err(err) => {
                            eprintln!(
                                "Warning: Invalid binding '{key_str}' for '{section_name}.{key_name}': {err}. Ignoring."
                            );
                            continue;
                        }
                    };

                    config.bindings.insert((mode, binding), action);
                }
            }
        }

        config.repeat_suppressed = build_repeat_suppressed(&config.bindings);
        Ok(config)
    }

    fn clear_action(&mut self, mode: InputMode, action: InputAction) {
        self.bindings
            .retain(|(m, _binding), a| !(*m == mode && *a == action));
    }
}

fn is_hint_reachable_binding(mode: InputMode, binding: &KeyBinding) -> bool {
    match mode {
        InputMode::Search => match binding.code {
            KeyCode::Backspace => false,
            KeyCode::Char(_) if !binding.modifiers.contains(KeyModifiers::CONTROL) => false,
            _ => true,
        },
        InputMode::Settings => !matches!(binding.code, KeyCode::Backspace),
        _ => true,
    }
}

#[derive(Debug, Clone, Copy)]
enum SectionKind {
    Normal,
    Search,
    Settings,
}

fn action_for_key(section: SectionKind, key: &str) -> Option<InputAction> {
    match section {
        SectionKind::Normal => match key {
            "quit" => Some(InputAction::Quit),
            "open_settings" => Some(InputAction::OpenSettings),
            "enter_search" => Some(InputAction::EnterSearch),
            "navigate_up" => Some(InputAction::NavigateUp),
            "navigate_down" => Some(InputAction::NavigateDown),
            "play_selected" => Some(InputAction::PlaySelected),
            "toggle_pause" => Some(InputAction::TogglePause),
            "next_track" => Some(InputAction::NextTrack),
            "prev_track" => Some(InputAction::PreviousTrack),
            "toggle_shuffle" => Some(InputAction::ToggleShuffle),
            "refresh" => Some(InputAction::Refresh),
            "cycle_sort" => Some(InputAction::CycleSort),
            _ => None,
        },
        SectionKind::Search => match key {
            "search_exit" => Some(InputAction::SearchExit),
            "toggle_pause" => Some(InputAction::TogglePause),
            "clear_line" => Some(InputAction::SearchClearLine),
            "navigate_up" => Some(InputAction::NavigateUp),
            "navigate_down" => Some(InputAction::NavigateDown),
            "play_selected" => Some(InputAction::PlaySelected),
            _ => None,
        },
        SectionKind::Settings => match key {
            "settings_close" => Some(InputAction::SettingsClose),
            "settings_confirm" => Some(InputAction::SettingsConfirm),
            "settings_left" => Some(InputAction::SettingsLeft),
            "settings_right" => Some(InputAction::SettingsRight),
            "clear_line" => Some(InputAction::SettingsClearLine),
            "navigate_up" => Some(InputAction::SettingsNavigateUp),
            "navigate_down" => Some(InputAction::SettingsNavigateDown),
            _ => None,
        },
    }
}

fn parse_key_list(value: &toml::Value) -> Result<Vec<String>, String> {
    match value {
        toml::Value::String(s) => Ok(vec![s.clone()]),
        toml::Value::Array(arr) => {
            let mut out = Vec::new();
            for item in arr {
                let Some(s) = item.as_str() else {
                    return Err("Expected an array of strings".to_string());
                };
                out.push(s.to_string());
            }
            Ok(out)
        }
        _ => Err("Expected a string or array of strings".to_string()),
    }
}

fn keymap_path(config_dir: &Path) -> PathBuf {
    config_dir.join(&format!("{}", APP_NAME)).join("keymap.toml")
}

fn write_default_keymap_if_missing(file_path: &Path) -> io::Result<()> {
    let Some(parent) = file_path.parent() else {
        return Ok(());
    };

    fs::create_dir_all(parent)?;

    let mut file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file_path)
    {
        Ok(f) => f,
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => return Ok(()),
        Err(err) => return Err(err),
    };

    file.write_all(default_keymap_toml().as_bytes())?;
    Ok(())
}

fn default_keymap_toml() -> String {
    let mut bindings: HashMap<(InputMode, InputAction), Vec<KeyBinding>> = HashMap::new();

    for (mode, binding, action) in defaults::default_bindings() {
        if !matches!(mode, InputMode::Normal | InputMode::Search | InputMode::Settings) {
            continue;
        }

        let list = bindings.entry((mode, action)).or_default();
        if !list.contains(&binding) {
            list.push(binding);
        }
    }

    let mut out = String::new();
    out.push_str(&format!("# {} keymap configuration\n", APP_NAME));
    out.push_str("#\n");
    out.push_str(&format!("# Generated automatically by {} (delete to regenerate).\n", APP_NAME));
    out.push_str("# Edit this file to customise key bindings.\n");
    out.push_str("# Omitted entries fall back to compiled-in defaults.\n");
    out.push_str("# Multiple keys for the same action: use an array.\n");
    out.push_str("# Single key: string or single-element array, both accepted.\n\n");

    write_section(
        &mut out,
        "normal",
        InputMode::Normal,
        &[
            (InputAction::Quit, "quit"),
            (InputAction::OpenSettings, "open_settings"),
            (InputAction::EnterSearch, "enter_search"),
            (InputAction::NavigateUp, "navigate_up"),
            (InputAction::NavigateDown, "navigate_down"),
            (InputAction::PlaySelected, "play_selected"),
            (InputAction::TogglePause, "toggle_pause"),
            (InputAction::NextTrack, "next_track"),
            (InputAction::PreviousTrack, "prev_track"),
            (InputAction::ToggleShuffle, "toggle_shuffle"),
            (InputAction::Refresh, "refresh"),
            (InputAction::CycleSort, "cycle_sort"),
        ],
        &bindings,
    );
    out.push('\n');

    write_section(
        &mut out,
        "search",
        InputMode::Search,
        &[
            (InputAction::SearchExit, "search_exit"),
            (InputAction::TogglePause, "toggle_pause"),
            (InputAction::SearchClearLine, "clear_line"),
            (InputAction::NavigateUp, "navigate_up"),
            (InputAction::NavigateDown, "navigate_down"),
            (InputAction::PlaySelected, "play_selected"),
        ],
        &bindings,
    );
    out.push('\n');

    write_section(
        &mut out,
        "settings",
        InputMode::Settings,
        &[
            (InputAction::SettingsClose, "settings_close"),
            (InputAction::SettingsConfirm, "settings_confirm"),
            (InputAction::SettingsLeft, "settings_left"),
            (InputAction::SettingsRight, "settings_right"),
            (InputAction::SettingsClearLine, "clear_line"),
            (InputAction::SettingsNavigateUp, "navigate_up"),
            (InputAction::SettingsNavigateDown, "navigate_down"),
        ],
        &bindings,
    );

    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn write_section(
    out: &mut String,
    section_name: &str,
    mode: InputMode,
    order: &[(InputAction, &str)],
    bindings: &HashMap<(InputMode, InputAction), Vec<KeyBinding>>,
) {
    out.push('[');
    out.push_str(section_name);
    out.push_str("]\n");

    for (action, toml_key) in order {
        let Some(keys) = bindings.get(&(mode, *action)) else {
            continue;
        };

        if keys.is_empty() {
            continue;
        }

        out.push_str(toml_key);
        out.push_str(" = ");

        if keys.len() == 1 {
            write_toml_string(out, &keys[0].to_string());
            out.push('\n');
            continue;
        }

        out.push('[');
        for (i, binding) in keys.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            write_toml_string(out, &binding.to_string());
        }
        out.push_str("]\n");
    }
}

fn write_toml_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
}

fn build_repeat_suppressed(
    bindings: &HashMap<(InputMode, KeyBinding), InputAction>,
) -> HashSet<(InputMode, KeyBinding)> {
    bindings
        .iter()
        .filter_map(|((mode, binding), action)| match action {
            InputAction::OpenSettings | InputAction::SettingsClose => Some((*mode, *binding)),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn b(code: KeyCode) -> KeyBinding {
        KeyBinding {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn ctrl(code: KeyCode) -> KeyBinding {
        KeyBinding {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    #[test]
    fn default_contains_expected_bindings() {
        let cfg = KeyConfig::default();

        assert_eq!(cfg.get(InputMode::Normal, &b(KeyCode::Char('q'))), Some(InputAction::Quit));
        assert_eq!(cfg.get(InputMode::Search, &b(KeyCode::Esc)), Some(InputAction::SearchExit));
        assert_eq!(
            cfg.get(InputMode::Settings, &b(KeyCode::Char('s'))),
            Some(InputAction::SettingsClose)
        );
    }

    #[test]
    fn load_or_default_absent_file_returns_default() {
        let dir = std::env::temp_dir().join(format!(
            "hextune_keyconfig_absent_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let cfg = KeyConfig::load_or_default(&dir);
        assert_eq!(cfg, KeyConfig::default());

        let file_path = keymap_path(&dir);
        assert!(file_path.exists());

        let toml_str = fs::read_to_string(&file_path).unwrap();
        let parsed = KeyConfig::parse(&toml_str).unwrap();
        assert_eq!(parsed, KeyConfig::default());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_or_default_invalid_toml_returns_default() {
        let dir = std::env::temp_dir().join(format!(
            "hextune_keyconfig_invalid_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let app_dir = dir.join("hextune");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("keymap.toml"), "this is not = toml").unwrap();

        let cfg = KeyConfig::load_or_default(&dir);
        assert_eq!(cfg, KeyConfig::default());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn user_override_replaces_defaults_for_action_in_mode() {
        let toml_str = r#"
[normal]
quit = "x"
"#;

        let cfg = KeyConfig::parse(toml_str).unwrap();
        assert_eq!(cfg.get(InputMode::Normal, &b(KeyCode::Char('x'))), Some(InputAction::Quit));
        assert_eq!(cfg.get(InputMode::Normal, &b(KeyCode::Char('q'))), None);
    }

    #[test]
    fn user_entry_does_not_affect_other_modes() {
        let toml_str = r#"
[normal]
quit = "x"
"#;
        let cfg = KeyConfig::parse(toml_str).unwrap();
        assert_eq!(cfg.get(InputMode::Search, &b(KeyCode::Char('x'))), None);
        assert_eq!(cfg.get(InputMode::Settings, &b(KeyCode::Char('x'))), None);
    }

    #[test]
    fn repeat_suppression_tracks_toggle_key_bindings() {
        let cfg = KeyConfig::default();

        assert!(cfg.is_repeat_suppressed(
            InputMode::Normal,
            &b(KeyCode::Char('s'))
        ));
        assert!(cfg.is_repeat_suppressed(
            InputMode::Settings,
            &b(KeyCode::Char('s'))
        ));
        assert!(cfg.is_repeat_suppressed(
            InputMode::Settings,
            &b(KeyCode::Esc)
        ));

        let override_toml = r#"
[normal]
open_settings = "o"
"#;
        let cfg2 = KeyConfig::parse(override_toml).unwrap();
        assert!(cfg2.is_repeat_suppressed(InputMode::Normal, &b(KeyCode::Char('o'))));
        assert!(!cfg2.is_repeat_suppressed(InputMode::Normal, &b(KeyCode::Char('s'))));
    }

    #[test]
    fn default_clear_line_bindings_exist() {
        let cfg = KeyConfig::default();
        assert_eq!(
            cfg.get(InputMode::Search, &ctrl(KeyCode::Char('u'))),
            Some(InputAction::SearchClearLine)
        );
        assert_eq!(
            cfg.get(InputMode::Settings, &ctrl(KeyCode::Char('u'))),
            Some(InputAction::SettingsClearLine)
        );
    }

    #[test]
    fn hint_bindings_filter_search_typing_keys() {
        let toml_str = r#"
[search]
navigate_down = ["j", "Down", "Ctrl+j", "Backspace"]
"#;
        let cfg = KeyConfig::parse(toml_str).unwrap();
        let bindings = cfg.hint_bindings_for_action(InputMode::Search, InputAction::NavigateDown);
        assert_eq!(
            bindings,
            vec![
                KeyBinding {
                    code: KeyCode::Char('j'),
                    modifiers: KeyModifiers::CONTROL,
                },
                KeyBinding {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE,
                },
            ]
        );
    }

    #[test]
    fn hint_bindings_filter_settings_backspace() {
        let toml_str = r#"
[settings]
settings_confirm = ["Backspace", "Enter"]
"#;
        let cfg = KeyConfig::parse(toml_str).unwrap();
        let bindings = cfg.hint_bindings_for_action(InputMode::Settings, InputAction::SettingsConfirm);
        assert_eq!(
            bindings,
            vec![KeyBinding {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            }]
        );
    }
}
