use crate::application::state::AppState;
use crate::core::traits::StorageBackend;
use crate::modules::storage::json_backend::JsonStorageBackend;
use crate::modules::ui::terminal::renderer::TerminalRenderer;
use anyhow::Result;
use crate::application::app::Application;
use crate::modules::playback::rodio_backend::RodioBackend;

pub struct CliContext {
    pub storage: JsonStorageBackend,
    pub state: AppState,
    pub ui: TerminalRenderer,
    pub backend: RodioBackend,
}

impl CliContext {
    pub fn load() -> Result<Self> {
        let storage = JsonStorageBackend::new()?;
        let state = storage.load()?;
        Ok(Self {
            storage,
            state,
            ui: TerminalRenderer::new(),
            backend: RodioBackend::new()?,
        })
    }

    pub fn new_app<T>(context: T) -> Result<Application> where T: Into<Option<CliContext>> {

        let ctx = match context.into() {
            Some(c) => c,
            None => CliContext::load()?,
        };

        Ok(Application::new()
            .with_playback_backend(Box::new(ctx.backend))
            .with_storage_backend(Box::new(ctx.storage))
            .with_ui_renderer(Box::new(ctx.ui)))
    }
}
