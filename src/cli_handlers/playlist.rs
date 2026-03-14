use crate::cli_handlers::CliCommand;
use crate::core::events::{AppEvent, PlaybackEvent};
use crate::utils::APP_NAME;
use anyhow::Result;
use crate::cli_handlers::context::CliContext;

pub struct PlaylistCommand;

impl CliCommand for PlaylistCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let ctx = CliContext::load()?;

        if ctx.state.library.songs.is_empty() {
            ctx.ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        let first_song = ctx.state.library.songs[0].clone();

        let mut app = CliContext::new_app(ctx)?;

        app.init()?;

        app.event_sender()
            .send(AppEvent::Playback(PlaybackEvent::PlayRequested { song: first_song }))?;

        app.run()?;
        app.cleanup()?;

        Ok(())
    }
}