use crate::application::app::Application;
use crate::cli_handlers::CliCommand;
use crate::core::events::{AppEvent, PlaybackEvent, UiEvent};
use crate::core::models::RepeatMode;
use crate::modules::playback::rodio_backend::RodioBackend;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use crate::utils::{amplitude_to_volume, volume_percent_to_amplitude};
use anyhow::Result;
use crate::core::traits::StorageBackend;

// ── Volume ────────────────────────────────────────────────────────────────────
pub struct VolumeCommand {
    pub volume: Option<u8>,
}

impl CliCommand for VolumeCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;
        let ui = TerminalRenderer::new();

        match self.volume {
            Some(vol) => {
                let volume_f32 = volume_percent_to_amplitude(vol);

                let mut app = Application::new()
                    .with_playback_backend(Box::new(RodioBackend::new()?))
                    .with_storage_backend(Box::new(storage))
                    .with_ui_renderer(Box::new(ui));

                app.init()?;
                app.event_sender()
                    .send(AppEvent::Playback(PlaybackEvent::VolumeChanged { volume: volume_f32 }))?;
                app.run_once()?;
                app.cleanup()?;

                let ui = TerminalRenderer::new();
                ui.print_message(&format!("Volume set to: {}%", vol));
            }
            None => {
                let current_percent = amplitude_to_volume(state.config.volume);
                ui.print_message(&format!("Current volume: {}%", current_percent));
            }
        }

        Ok(())
    }
}

// ── Shuffle ───────────────────────────────────────────────────────────────────
pub struct ShuffleCommand {
    pub enabled: Option<bool>,
}

impl CliCommand for ShuffleCommand {
    fn execute(self :Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;
        let ui = TerminalRenderer::new();

        let new_state = self.enabled.unwrap_or(!state.config.shuffle);

        let mut app = Application::new()
            .with_playback_backend(Box::new(RodioBackend::new()?))
            .with_storage_backend(Box::new(storage))
            .with_ui_renderer(Box::new(ui));

        app.init()?;
        app.event_sender()
            .send(AppEvent::Ui(UiEvent::ShuffleSet { enabled: new_state }))?;
        app.run_once()?;
        app.cleanup()?;

        let ui = TerminalRenderer::new();
        ui.print_message(&format!("Shuffle set to: {}", new_state));

        Ok(())
    }
}

// ── Loop ─────────────────────────────────────────────────────────────
pub struct LoopCommand {
    pub mode: Option<RepeatMode>,
}

impl CliCommand for LoopCommand {
    fn execute(self: Box<Self>) -> Result<()> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;
        let ui = TerminalRenderer::new();

        let new_mode = self.mode.unwrap_or_else(|| state.config.repeat.cycle());

        let mut app = Application::new()
            .with_playback_backend(Box::new(RodioBackend::new()?))
            .with_storage_backend(Box::new(storage))
            .with_ui_renderer(Box::new(ui));

        app.init()?;
        app.event_sender()
            .send(AppEvent::Playback(PlaybackEvent::RepeatChanged { mode: new_mode }))?;
        app.run_once()?;
        app.cleanup()?;

        let ui = TerminalRenderer::new();
        ui.print_message(&format!(
            "Repeat mode set to: {} {}",
            new_mode.symbol(),
            repeat_mode_description(new_mode),
        ));

        Ok(())
    }
}

/// Human-readable label used in terminal feedback messages
fn repeat_mode_description(mode: RepeatMode) -> &'static str {
    match mode {
        RepeatMode::Off => "(stop at end)",
        RepeatMode::All => "(loop playlist)",
        RepeatMode::One => "(repeat current song)",
    }
}