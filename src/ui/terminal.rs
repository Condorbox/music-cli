
use std::io::{stdout, Write};
use crossterm::{
    cursor,
    terminal::{self, ClearType},
    ExecutableCommand,
};
use crate::models::Song;
use super::Ui;

pub struct TerminalUi;

impl TerminalUi {
    pub fn new() -> Self {
        Self
    }

    fn format_duration(seconds: u64) -> String {
        let mins = seconds / 60;
        let secs = seconds % 60;
        format!("{}:{:02}", mins, secs)
    }
}

impl Ui for TerminalUi {
    fn show_status(&mut self, is_paused: bool, song: &Song) {
        let mut stdout = stdout();

        stdout.execute(cursor::MoveToColumn(0)).ok();
        stdout.execute(terminal::Clear(ClearType::CurrentLine)).ok();

        // Build metadata string
        let mut info_parts = vec![song.title.clone()];

        if let Some(artist) = &song.artist {
            info_parts.push(format!("by {}", artist));
        }

        if let Some(album) = &song.album {
            info_parts.push(format!("from '{}'", album));
        }

        if let Some(duration) = song.duration {
            info_parts.push(format!("[{}]", Self::format_duration(duration)));
        }

        let metadata_str = info_parts.join(" ");

        print!(
            "{} | {} | [Space/P/K: Pause/Play | Q/Esc: Quit | N/Right: Next | B/Left: Back]",
            if is_paused { "⏸ Paused " } else { "▶ Playing" },
            metadata_str
        );

        stdout.flush().ok();
    }


    fn clear_status(&mut self) {
        let mut stdout = stdout();
        stdout.execute(cursor::MoveToColumn(0)).ok();
        stdout.execute(terminal::Clear(ClearType::CurrentLine)).ok();
        stdout.flush().ok();
    }

    fn print_message(&mut self, message: &str) {
        self.clear_status();
        println!("{}", message);
    }

    fn print_error(&mut self, message: &str) {
        eprint!("{}", message);
    }
}
