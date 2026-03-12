use crate::cli_handlers::CliCommand;
use crate::core::traits::StorageBackend;
use crate::modules::library::sorter::{sort_songs, SortField};
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::utils::APP_NAME;
use anyhow::Result;

pub struct SortCommand {
    pub field: SortField,
}

impl CliCommand for SortCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;
        let ui = TerminalRenderer::new();

        if state.library.songs.is_empty() {
            ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        let sorted = sort_songs(&state.library.songs, self.field);
        ui.print_song_list_refs(&sorted);

        Ok(())
    }
}