use crate::cli_handlers::CliCommand;
use crate::modules::library::scanner;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::core::traits::StorageBackend;
use crate::utils::APP_NAME;
use anyhow::Result;

pub struct RefreshCommand;

impl CliCommand for RefreshCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let mut state = storage.load()?;
        let ui = TerminalRenderer::new();

        let root_path = state.config.root_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(
                "No music path set. Run '{} path <DIR>' first.", APP_NAME
            ))?
            .clone();

        ui.print_message(&format!("Scanning {:?}...", root_path));

        let songs = scanner::scan_directory(&root_path)?;
        let count = songs.len();

        state.library.songs = songs;
        storage.save(&state)?;

        ui.print_message(&format!("✓ Refresh complete. Found {} songs.", count));

        Ok(())
    }
}