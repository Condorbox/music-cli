use crate::cli_handlers::CliCommand;
use crate::utils::APP_NAME;
use anyhow::Result;
use crate::cli_handlers::context::CliContext;

pub struct ListCommand;

impl CliCommand for ListCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let ctx = CliContext::load()?;

        if ctx.state.library.songs.is_empty() {
            ctx.ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        ctx.ui.print_song_list(&ctx.state.library.songs);

        Ok(())
    }
}