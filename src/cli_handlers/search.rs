use crate::cli_handlers::context::CliContext;
use crate::cli_handlers::CliCommand;
use crate::modules::library::search_engine::SearchEngine;
use crate::utils::APP_NAME;
use anyhow::Result;

pub struct SearchCommand {
    pub query: String,
}

impl CliCommand for SearchCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let ctx = CliContext::load()?;

        if ctx.state.library.songs.is_empty() {
            ctx.ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        let search_engine = SearchEngine::new();
        let results = search_engine.search(&ctx.state.library.songs, &self.query);
        let indexed = search_engine.search_result_to_song_index(results);

        ctx.ui.print_search_results(&self.query, &indexed);

        Ok(())
    }
}