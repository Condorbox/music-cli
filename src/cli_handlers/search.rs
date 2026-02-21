use crate::cli_handlers::CliCommand;
use crate::modules::library::search_engine::SearchEngine;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::core::traits::StorageBackend;
use crate::utils::APP_NAME;
use anyhow::Result;

pub struct SearchCommand {
    pub query: String,
}

impl CliCommand for SearchCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;
        let ui = TerminalRenderer::new();

        if state.library.songs.is_empty() {
            ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        let search_engine = SearchEngine::new();
        let results = search_engine.search(&state.library.songs, &self.query);
        let indexed = search_engine.search_result_to_song_index(results);

        ui.print_search_results(&self.query, &indexed);

        Ok(())
    }
}