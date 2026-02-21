use crate::cli_handlers::CliCommand;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::core::traits::StorageBackend;
use crate::utils::APP_NAME;
use anyhow::Result;
use std::path::PathBuf;

pub struct PathCommand {
    pub directory: PathBuf,
}

// TODO Maybe refresh too
impl CliCommand for PathCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let mut state = storage.load()?;
        let ui = TerminalRenderer::new();

        let path = self.directory.canonicalize()?;
        if !path.is_dir() {
            anyhow::bail!("The path provided is not a valid directory.");
        }

        state.config.root_path = Some(path.clone());
        storage.save(&state)?;

        ui.print_message(&format!("Music path updated to: {:?}", path));
        ui.print_message(&format!("Run '{} refresh' to scan for music files.", APP_NAME));

        Ok(())
    }
}