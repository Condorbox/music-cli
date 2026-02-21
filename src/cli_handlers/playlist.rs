use crate::application::app::Application;
use crate::cli_handlers::CliCommand;
use crate::core::events::{AppEvent, PlaybackEvent};
use crate::modules::playback::rodio_backend::RodioBackend;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::utils::APP_NAME;
use anyhow::Result;
use crate::core::traits::StorageBackend;

pub struct PlaylistCommand;

impl CliCommand for PlaylistCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;
        let ui = TerminalRenderer::new();

        if state.library.songs.is_empty() {
            ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        ui.print_message(&format!("Queueing {} songs...\n", state.library.songs.len()));

        let mut app = Application::new()
            .with_playback_backend(Box::new(RodioBackend::new()?))
            .with_storage_backend(Box::new(storage))
            .with_ui_renderer(Box::new(TerminalRenderer::new()));

        // Set up playlist in state
        {
            let mut app_state = app.state();
            app_state.playback.playlist = state.library.songs.clone();
            app_state.playback.current_index = Some(0);
            app_state.library.songs = state.library.songs.clone();
        }

        app.init()?;

        let first_song = state.library.songs[0].clone();
        app.event_sender()
            .send(AppEvent::Playback(PlaybackEvent::PlayRequested { song: first_song }))?;

        app.run()?;
        app.cleanup()?;

        Ok(())
    }
}