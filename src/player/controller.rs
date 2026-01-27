use anyhow::Result;

use crate::player::audio_player::AudioPlayer;
use crate::player::tui_controller::TuiController;
use crate::ui::tui::TuiUi;

pub fn run_tui_player(ui: &mut TuiUi) -> Result<()> {
    let mut player = AudioPlayer::new()?;
    TuiController::run(ui, &mut player)
}