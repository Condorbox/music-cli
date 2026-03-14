use crate::cli_handlers::CliCommand;
use crate::core::traits::StorageBackend;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::utils::{amplitude_to_volume, repeat_label, APP_NAME};
use anyhow::Result;
use crate::cli_handlers::context::CliContext;

pub struct StatusCommand;

impl CliCommand for StatusCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let ctx = CliContext::load()?;

        let volume = amplitude_to_volume(ctx.state.config.volume);
        let shuffle = if ctx.state.config.shuffle { "On" } else { "Off" };
        let repeat = format!(
            "{} {}",
            ctx.state.config.repeat.symbol(),
            repeat_label(ctx.state.config.repeat)
        );
        let song_count = ctx.state.library.songs.len();
        let library_path = ctx.state
            .config
            .root_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(not set)".to_string());

        ctx.ui.print_message(&format!("─── {} ──────────────────────────", APP_NAME));
        ctx.ui.print_message(&format!("  Volume   {}%", volume));
        ctx.ui.print_message(&format!("  Shuffle  {}", shuffle));
        ctx.ui.print_message(&format!("  Repeat   {}", repeat));
        ctx.ui.print_message("────────────────────────────────────────");
        ctx.ui.print_message(&format!("  Library  {} songs", song_count));
        ctx.ui.print_message(&format!("  Path     {}", library_path));
        ctx.ui.print_message("────────────────────────────────────────");

        Ok(())
    }
}