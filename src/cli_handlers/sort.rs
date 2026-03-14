use crate::cli_handlers::CliCommand;
use crate::modules::library::sorter::{sort_songs, SortField};
use crate::utils::APP_NAME;
use anyhow::Result;
use crate::cli_handlers::context::CliContext;

pub struct SortCommand {
    pub field: SortField,
}

impl CliCommand for SortCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let ctx = CliContext::load()?;

        if ctx.state.library.songs.is_empty() {
            ctx.ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        let sorted = sort_songs(&ctx.state.library.songs, self.field);
        ctx.ui.print_song_list_refs(&sorted);

        Ok(())
    }
}