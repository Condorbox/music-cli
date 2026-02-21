use crate::cli_handlers::CliCommand;
use crate::core::models::Song;
use crate::core::traits::PlaybackBackend;
use crate::modules::playback::rodio_backend::RodioBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use anyhow::Result;
use std::path::PathBuf;

pub struct PlayCommand {
    pub file: PathBuf,
}

impl CliCommand for PlayCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let song = Song::from_path(&self.file);
        let ui = TerminalRenderer::new();

        ui.print_message(&format!("Playing: {}", song.title));

        let mut backend = RodioBackend::new()?;
        backend.play(&song)?;

        ui.print_message("Press Ctrl+C to stop");
        while backend.is_playing() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        ui.print_message("✓ Playback finished");

        Ok(())
    }
}