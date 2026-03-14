use crate::modules::ui::tui::settings_state::{PathValidation, SettingsField, SettingsState};
use crate::utils::repeat_label;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, settings: &SettingsState) {
    let height_pct = if settings.is_editing_path() { 60 } else { 50 };
    let area = centered_rect(60, height_pct, f.area());
    f.render_widget(Clear, area);
    f.render_widget(
        Block::default()
            .title(" ⚙ Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
        area,
    );

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(4),
    };

    let path_error_height = match settings.path_validation() {
        PathValidation::Error(_) => 1,
        PathValidation::Idle => 0,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),                 // Volume
            Constraint::Length(3),                 // Repeat
            Constraint::Length(3),                 // Music Path input
            Constraint::Length(path_error_height), // Inline error (0 or 1)
            Constraint::Min(0),                    // spacer
            Constraint::Length(2),                 // help
        ])
        .split(inner);

    draw_volume(f, settings, chunks[0]);
    draw_repeat(f, settings, chunks[1]);
    draw_path(f, settings, chunks[2]);
    draw_path_error(f, settings, chunks[3]);
    draw_help(f, settings, chunks[5]);
}

fn draw_volume(f: &mut Frame, settings: &SettingsState, area: Rect) {
    let selected = settings.selected() == SettingsField::Volume;
    let editing = selected && settings.is_editing_volume();

    let label = if editing {
        format!(
            "Volume: {}%  [←/→ adjust • 0-9 type • Enter confirm • Esc cancel]",
            settings.temp_volume()
        )
    } else {
        format!("Volume: {}%", settings.temp_volume())
    };

    f.render_widget(Paragraph::new(label).style(field_style(selected)), area);
}

fn draw_repeat(f: &mut Frame, settings: &SettingsState, area: Rect) {
    let selected = settings.selected() == SettingsField::Repeat;
    let temp_repeat = settings.temp_repeat();

    let label = if selected {
        format!(
            "Repeat: {} {}  [←/→ or Enter to cycle]",
            temp_repeat.symbol(),
            repeat_label(temp_repeat),
        )
    } else {
        format!(
            "Repeat: {} {}",
            temp_repeat.symbol(),
            repeat_label(temp_repeat),
        )
    };

    f.render_widget(Paragraph::new(label).style(field_style(selected)), area);
}

fn draw_path(f: &mut Frame, settings: &SettingsState, area: Rect) {
    let selected = settings.selected() == SettingsField::MusicPath;

    let label_color = if selected { Color::Yellow } else { Color::White };
    let hint_color = if selected { Color::Yellow } else { Color::DarkGray };

    let label: Line = if settings.is_editing_path() {
        Line::from(vec![
            Span::styled(
                "Music Path: ",
                Style::default()
                    .fg(label_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(settings.temp_path(), Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "  [Enter confirm • Esc cancel • Ctrl+U clear]",
                Style::default().fg(Color::Yellow),
            ),
        ])
    } else if settings.temp_path().is_empty() {
        let mut spans = vec![
            Span::styled("Music Path: ", Style::default().fg(label_color)),
            Span::styled("(not set)", Style::default().fg(Color::DarkGray)),
        ];
        if selected {
            spans.push(Span::styled("  [Enter to set]", Style::default().fg(hint_color)));
        }
        Line::from(spans)
    } else {
        let mut spans = vec![
            Span::styled("Music Path: ", Style::default().fg(label_color)),
            Span::styled(settings.temp_path(), Style::default().fg(Color::Cyan)),
        ];
        if selected {
            spans.push(Span::styled(
                "  [Enter to change]",
                Style::default().fg(hint_color),
            ));
        }
        Line::from(spans)
    };

    f.render_widget(Paragraph::new(label), area);
}

fn draw_path_error(f: &mut Frame, settings: &SettingsState, area: Rect) {
    if let PathValidation::Error(msg) = settings.path_validation() {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "  ✗ ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(msg.as_str(), Style::default().fg(Color::Red)),
            ])),
            area,
        );
    }
}

fn draw_help(f: &mut Frame, settings: &SettingsState, area: Rect) {
    let text = if settings.is_editing_volume() {
        "←/→: Adjust  •  0-9: Type value  •  Enter: Confirm  •  Esc: Cancel"
    } else if settings.is_editing_path() {
        "Type path  •  Enter: Confirm  •  Esc: Cancel  •  Ctrl+U: Clear"
    } else {
        match settings.selected() {
            SettingsField::Volume => "↑/↓: Navigate  •  Enter: Edit volume  •  s/Esc: Close",
            SettingsField::Repeat => {
                "↑/↓: Navigate  •  ←/→ or Enter: Cycle mode  •  s/Esc: Close"
            }
            SettingsField::MusicPath => "↑/↓: Navigate  •  Enter: Edit path  •  s/Esc: Close",
        }
    };

    f.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        area,
    );
}

fn field_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

