use crate::modules::input::{InputAction, InputMode, KeyBinding, KeyConfig};
use crossterm::event::{KeyCode, KeyModifiers};

pub  fn kb(code: KeyCode) -> KeyBinding {
    KeyBinding {
        code,
        modifiers: KeyModifiers::NONE,
    }
}

pub fn kb_ctrl_char(c: char) -> KeyBinding {
    KeyBinding {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::CONTROL,
    }
}

pub fn ordered_bindings_with_preference(
    config: &KeyConfig,
    mode: InputMode,
    action: InputAction,
    preferred_order: &[KeyBinding],
) -> Vec<KeyBinding> {
    let bindings = config.hint_bindings_for_action(mode, action);
    if bindings.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();

    for preferred in preferred_order {
        if bindings.contains(preferred) && !out.contains(preferred) {
            out.push(*preferred);
        }
    }

    for binding in bindings {
        if !out.contains(&binding) {
            out.push(binding);
        }
    }

    out
}

pub fn pick_binding_with_preference(
    config: &KeyConfig,
    mode: InputMode,
    action: InputAction,
    preferred: &[KeyBinding],
) -> Option<KeyBinding> {
    ordered_bindings_with_preference(config, mode, action, preferred)
        .first()
        .copied()
}

pub fn format_binding(binding: KeyBinding) -> String {
    let s = binding.to_string();
    match s.as_str() {
        "Up" => "↑".to_string(),
        "Down" => "↓".to_string(),
        "Left" => "←".to_string(),
        "Right" => "→".to_string(),
        _ => s.replace("+Up", "+↑")
            .replace("+Down", "+↓")
            .replace("+Left", "+←")
            .replace("+Right", "+→"),
    }
}

pub fn format_binding_opt(binding: Option<KeyBinding>) -> String {
    binding.map(format_binding).unwrap_or_else(|| "—".to_string())
}

pub fn format_bindings_join(bindings: &[KeyBinding]) -> String {
    bindings
        .iter()
        .copied()
        .map(format_binding)
        .collect::<Vec<_>>()
        .join("/")
}

