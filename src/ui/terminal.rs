
use std::io::{stdout, Write};
use crossterm::{
    cursor,
    terminal::{self, ClearType},
    ExecutableCommand,
};

use super::Ui;

pub struct TerminalUi;

impl TerminalUi {
    pub fn new() -> Self {
        Self
    }
}

impl Ui for TerminalUi {
    fn show_status(&mut self, is_paused: bool, current_info: &str) {
        let mut stdout = stdout();

        stdout.execute(cursor::MoveToColumn(0)).ok();
        stdout.execute(terminal::Clear(ClearType::CurrentLine)).ok();

        print!(
            "{} | {} | [Space/P/K: Pause/Play | Q/Esc: Quit | N/Right: Next | B/Left: Back]",
            if is_paused { "⏸ Paused " } else { "▶ Playing" },
            current_info
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
