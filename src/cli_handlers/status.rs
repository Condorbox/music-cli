use crate::cli_handlers::CliCommand;
use crate::core::traits::StorageBackend;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::utils::{amplitude_to_volume, repeat_label, APP_NAME};
use anyhow::Result;

pub struct StatusCommand;

impl CliCommand for StatusCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;
        let ui = TerminalRenderer::new();

        let volume = amplitude_to_volume(state.config.volume);
        let shuffle = if state.config.shuffle { "On" } else { "Off" };
        let repeat = format!(
            "{} {}",
            state.config.repeat.symbol(),
            repeat_label(state.config.repeat)
        );
        let song_count = state.library.songs.len();
        let library_path = state
            .config
            .root_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(not set)".to_string());

        ui.print_message(&format!("─── {} ──────────────────────────", APP_NAME));
        ui.print_message(&format!("  Volume   {}%", volume));
        ui.print_message(&format!("  Shuffle  {}", shuffle));
        ui.print_message(&format!("  Repeat   {}", repeat));
        ui.print_message("────────────────────────────────────────");
        ui.print_message(&format!("  Library  {} songs", song_count));
        ui.print_message(&format!("  Path     {}", library_path));
        ui.print_message("────────────────────────────────────────");

        Ok(())
    }
}