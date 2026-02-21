use crate::cli_handlers::CliCommand;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::core::traits::StorageBackend;
use crate::utils::APP_NAME;
use anyhow::Result;

pub struct ListCommand;

impl CliCommand for ListCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;
        let ui = TerminalRenderer::new();

        if state.library.songs.is_empty() {
            ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        ui.print_song_list(&state.library.songs);

        Ok(())
    }
}