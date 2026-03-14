use crate::cli_handlers::context::CliContext;
use crate::cli_handlers::CliCommand;
use crate::core::models::Song;
use crate::core::traits::PlaybackBackend;
use anyhow::Result;
use std::path::PathBuf;

pub struct PlayCommand {
    pub file: PathBuf,
}

impl CliCommand for PlayCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let mut ctx = CliContext::load()?;
        let song = Song::from_path(&self.file);

        ctx.ui.print_message(&format!("Playing: {}", song.title));

        ctx.backend.set_volume(ctx.state.config.volume);
        ctx.backend.play(&song)?;

        ctx.ui.print_message("Press Ctrl+C to stop");
        while ctx.backend.is_playing() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        ctx.ui.print_message("✓ Playback finished");

        Ok(())
    }
}