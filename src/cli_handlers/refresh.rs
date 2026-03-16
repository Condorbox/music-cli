use crate::cli_handlers::context::CliContext;
use crate::cli_handlers::CliCommand;
use crate::core::traits::StorageBackend;
use crate::modules::library::scanner;
use crate::utils::APP_NAME;
use anyhow::Result;
use std::sync::Arc;

pub struct RefreshCommand;

impl CliCommand for RefreshCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let mut ctx = CliContext::load()?;

        let root_path = ctx.state.config.root_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(
                "No music path set. Run '{} path <DIR>' first.", APP_NAME
            ))?
            .clone();

        ctx.ui.print_message(&format!("Scanning {:?}...", root_path));

        let songs = scanner::scan_directory(&root_path, |_| {})?;
        let count = songs.len();

        ctx.state.library.songs = Arc::new(songs);
        ctx.storage.save(&ctx.state)?;

        ctx.ui.print_message(&format!("✓ Refresh complete. Found {} songs.", count));

        Ok(())
    }
}