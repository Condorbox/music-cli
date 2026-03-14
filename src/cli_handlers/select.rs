use crate::cli_handlers::context::CliContext;
use crate::cli_handlers::CliCommand;
use crate::core::traits::PlaybackBackend;
use crate::utils::APP_NAME;
use crate::utils::CLI_PLAYBACK_POLL_MS;
use anyhow::Result;

pub struct SelectCommand {
    pub index: usize,
}

impl CliCommand for SelectCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let mut ctx = CliContext::load()?;

        if ctx.state.library.songs.is_empty() {
            ctx.ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
            return Ok(());
        }

        let song = ctx.state.library.songs.get(self.index)
            .ok_or_else(|| anyhow::anyhow!(
                "Invalid index {}. Library has {} songs (0-{}).",
                self.index,
                ctx.state.library.songs.len(),
                ctx.state.library.songs.len() - 1
            ))?;

        ctx.ui.print_message(&format!("Playing: {}", song.title));

        ctx.backend.set_volume(ctx.state.config.volume);
        ctx.backend.play(song)?;

        ctx.ui.print_message("Press Ctrl+C to stop");
        while ctx.backend.is_playing() {
            std::thread::sleep(std::time::Duration::from_millis(CLI_PLAYBACK_POLL_MS));
        }

        ctx.ui.print_message("✓ Playback finished");

        Ok(())
    }
}