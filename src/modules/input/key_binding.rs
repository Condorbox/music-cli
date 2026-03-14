use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fmt;
use std::str::FromStr;

/// A hashable, comparable representation of a single key press.
///
/// Strips `KeyEventKind` — binding lookup is kind-agnostic.
/// The kind filtering (drop Release, suppress Repeat on toggle keys)
/// stays in `handler.rs` where it already lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    /// Constructs a normalised binding from a crossterm `KeyEvent`.
    ///
    /// For `KeyCode::Char`, uppercase is normalised to lowercase and `SHIFT` is stripped,
    /// matching the existing handler behaviour where `'S'` and `'s'` are treated identically.
    pub fn from_event(key: KeyEvent) -> Self {
        let (code, modifiers) = normalize_char_key(key.code, key.modifiers);
        Self { code, modifiers }
    }

    /// Parses a human-readable string like `Ctrl+c`, `Alt+x`, `F5`, `Esc`, `Space`.
    pub fn from_str(s: &str) -> Result<Self, String> {
        let raw = s.trim();
        if raw.is_empty() {
            return Err("Key binding cannot be empty".to_string());
        }

        let parts: Vec<&str> = raw.split('+').map(|p| p.trim()).collect();
        if parts.iter().any(|p| p.is_empty()) {
            return Err(format!("Invalid key binding '{raw}': empty segment"));
        }

        let (key_part, modifier_parts) = parts
            .split_last()
            .ok_or_else(|| "Key binding cannot be empty".to_string())?;

        let mut modifiers = KeyModifiers::NONE;
        for m in modifier_parts {
            modifiers |= parse_modifier(m)?;
        }

        let code = parse_key_code(key_part)?;
        let (code, modifiers) = normalize_char_key(code, modifiers);

        Ok(Self { code, modifiers })
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<&'static str> = Vec::new();

        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift");
        }

        let key = display_key_code(self.code);
        if parts.is_empty() {
            write!(f, "{key}")
        } else {
            write!(f, "{}+{key}", parts.join("+"))
        }
    }
}

fn normalize_char_key(code: KeyCode, modifiers: KeyModifiers) -> (KeyCode, KeyModifiers) {
    match code {
        KeyCode::Char(c) => {
            let c = if c.is_ascii_alphabetic() {
                c.to_ascii_lowercase()
            } else {
                c
            };

            // For character keys, the shifted state is represented by the char itself (`!`, `?`, etc).
            // Keeping SHIFT would make bindings unexpectedly not match normal key presses.
            let modifiers = modifiers.difference(KeyModifiers::SHIFT);

            (KeyCode::Char(c), modifiers)
        }
        other => (other, modifiers),
    }
}

fn parse_modifier(s: &str) -> Result<KeyModifiers, String> {
    match s.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Ok(KeyModifiers::CONTROL),
        "alt" => Ok(KeyModifiers::ALT),
        "shift" => Ok(KeyModifiers::SHIFT),
        other => Err(format!("Unknown modifier '{other}'")),
    }
}

fn parse_key_code(s: &str) -> Result<KeyCode, String> {
    let raw = s.trim();
    if raw.is_empty() {
        return Err("Missing key name".to_string());
    }

    let lower = raw.to_ascii_lowercase();

    let code = match lower.as_str() {
        "esc" | "escape" => KeyCode::Esc,
        "enter" | "return" => KeyCode::Enter,
        "space" => KeyCode::Char(' '),
        "backspace" => KeyCode::Backspace,
        "tab" => KeyCode::Tab,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        _ if lower.len() >= 2
            && lower.starts_with('f')
            && lower[1..].chars().all(|c| c.is_ascii_digit()) =>
        {
            let n = u8::from_str(&lower[1..])
                .map_err(|_| format!("Invalid function key '{raw}'"))?;
            if n == 0 {
                return Err(format!("Invalid function key '{raw}'"));
            }
            KeyCode::F(n)
        }
        _ => {
            let mut chars = raw.chars();
            let Some(ch) = chars.next() else {
                return Err("Missing key name".to_string());
            };
            if chars.next().is_some() {
                return Err(format!("Unknown key name '{raw}'"));
            }
            KeyCode::Char(ch)
        }
    };

    Ok(code)
}

fn display_key_code(code: KeyCode) -> String {
    match code {
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::Char(c) => c.to_string(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::input::defaults::default_bindings;

    #[test]
    fn round_trip_all_default_bindings() {
        for (_mode, binding, _action) in default_bindings() {
            let parsed = KeyBinding::from_str(&binding.to_string())
                .unwrap_or_else(|e| panic!("Failed to parse '{}': {e}", binding));
            assert_eq!(parsed, binding);
        }
    }

    #[test]
    fn parses_common_bindings() {
        let ctrl_c = KeyBinding::from_str("Ctrl+c").unwrap();
        assert_eq!(ctrl_c.code, KeyCode::Char('c'));
        assert_eq!(ctrl_c.modifiers, KeyModifiers::CONTROL);

        let alt_x = KeyBinding::from_str("Alt+x").unwrap();
        assert_eq!(alt_x.code, KeyCode::Char('x'));
        assert_eq!(alt_x.modifiers, KeyModifiers::ALT);

        let f5 = KeyBinding::from_str("F5").unwrap();
        assert_eq!(f5.code, KeyCode::F(5));
        assert_eq!(f5.modifiers, KeyModifiers::NONE);

        let esc = KeyBinding::from_str("Esc").unwrap();
        assert_eq!(esc.code, KeyCode::Esc);
        assert_eq!(esc.modifiers, KeyModifiers::NONE);

        let space = KeyBinding::from_str("Space").unwrap();
        assert_eq!(space.code, KeyCode::Char(' '));
        assert_eq!(space.modifiers, KeyModifiers::NONE);

        let enter = KeyBinding::from_str("Enter").unwrap();
        assert_eq!(enter.code, KeyCode::Enter);
        assert_eq!(enter.modifiers, KeyModifiers::NONE);

        let slash = KeyBinding::from_str("/").unwrap();
        assert_eq!(slash.code, KeyCode::Char('/'));
        assert_eq!(slash.modifiers, KeyModifiers::NONE);

        let backspace = KeyBinding::from_str("Backspace").unwrap();
        assert_eq!(backspace.code, KeyCode::Backspace);
        assert_eq!(backspace.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn unknown_key_name_is_error() {
        let err = KeyBinding::from_str("NotAKey").unwrap_err();
        assert!(err.to_ascii_lowercase().contains("unknown"));
    }

    #[test]
    fn modifier_parsing_is_case_insensitive() {
        let a = KeyBinding::from_str("ctrl+C").unwrap();
        let b = KeyBinding::from_str("CTRL+C").unwrap();
        let c = KeyBinding::from_str("Ctrl+C").unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a.code, KeyCode::Char('c'));
        assert_eq!(a.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn from_event_normalises_uppercase_and_strips_shift_for_chars() {
        let key = KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT);
        let binding = KeyBinding::from_event(key);
        assert_eq!(
            binding,
            KeyBinding {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::NONE
            }
        );
    }
}
