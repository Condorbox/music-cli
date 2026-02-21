use crate::application::app::Application;
use crate::cli_handlers::CliCommand;
use crate::modules::playback::rodio_backend::RodioBackend;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::modules::ui::tui::renderer::TuiRenderer;
use crate::utils::APP_NAME;
use anyhow::Result;
use crate::core::traits::StorageBackend;

pub struct BrowseCommand;

impl CliCommand for BrowseCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;

        if state.library.songs.is_empty() {
            let ui = TerminalRenderer::new();
            ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        let mut tui_renderer = TuiRenderer::new();
        tui_renderer.set_songs(state.library.songs.clone());

        let mut app = Application::new()
            .with_playback_backend(Box::new(RodioBackend::new()?))
            .with_storage_backend(Box::new(storage))
            .with_ui_renderer(Box::new(tui_renderer));

        app.init()?;
        app.run()?;
        app.cleanup()?;

        Ok(())
    }
}