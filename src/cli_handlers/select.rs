use crate::cli_handlers::CliCommand;
use crate::core::traits::{PlaybackBackend, StorageBackend};
use crate::modules::playback::rodio_backend::RodioBackend;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::utils::APP_NAME;
use anyhow::Result;

pub struct SelectCommand {
    pub index: usize,
}

impl CliCommand for SelectCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;
        let ui = TerminalRenderer::new();

        if state.library.songs.is_empty() {
            ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        let song = state.library.songs.get(self.index)
            .ok_or_else(|| anyhow::anyhow!(
                "Invalid index {}. Library has {} songs (0-{}).",
                self.index,
                state.library.songs.len(),
                state.library.songs.len() - 1
            ))?;

        ui.print_message(&format!("Playing: {}", song.title));

        let mut backend = RodioBackend::new()?;
        backend.play(song)?;

        ui.print_message("Press Ctrl+C to stop");
        while backend.is_playing() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        ui.print_message("✓ Playback finished");

        Ok(())
    }
}