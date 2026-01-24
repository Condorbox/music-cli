use crate::models::Song;

pub mod terminal;

pub trait Ui {
    fn show_status(&mut self, is_paused: bool, song: &Song);
    fn clear_status(&mut self);

    fn print_message(&mut self, message: &str);
    fn print_error(&mut self, message: &str);
}
