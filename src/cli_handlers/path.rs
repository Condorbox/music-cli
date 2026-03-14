use crate::cli_handlers::CliCommand;
use crate::core::traits::StorageBackend;
use crate::utils::APP_NAME;
use anyhow::Result;
use std::path::PathBuf;
use crate::cli_handlers::context::CliContext;

pub struct PathCommand {
    pub directory: PathBuf,
}

impl CliCommand for PathCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let mut ctx = CliContext::load()?;

        let path = self.directory.canonicalize()?;
        if !path.is_dir() {
            anyhow::bail!("The path provided is not a valid directory.");
        }

        ctx.state.config.root_path = Some(path.clone());
        ctx.storage.save(&ctx.state)?;

        ctx.ui.print_message(&format!("Music path updated to: {:?}", path));
        ctx.ui.print_message(&format!("Run '{} refresh' to scan for music files.", APP_NAME));

        Ok(())
    }
}