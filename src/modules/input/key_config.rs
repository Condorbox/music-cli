use super::{defaults, InputAction, InputMode, KeyBinding};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

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
        let Ok(toml_str) = fs::read_to_string(&file_path) else {
            // Absent file is silent. Read errors fall back silently as well.
            return Self::default();
        };

        match Self::parse(&toml_str) {
            Ok(config) => config,
            Err(err) => {
                eprintln!(
                    "Warning: Failed to parse keymap at '{}': {err}. Using defaults.",
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
    config_dir.join("music-cli").join("keymap.toml")
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
            "music_cli_keyconfig_absent_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let cfg = KeyConfig::load_or_default(&dir);
        assert_eq!(cfg, KeyConfig::default());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_or_default_invalid_toml_returns_default() {
        let dir = std::env::temp_dir().join(format!(
            "music_cli_keyconfig_invalid_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let app_dir = dir.join("music-cli");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(app_dir.join("keymap.toml"), "this is not = toml").unwrap();

        let cfg = KeyConfig::load_or_default(&dir);
        assert_eq!(cfg, KeyConfig::default());

        let _ = std::fs::remove_dir_all(&dir);
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
}
