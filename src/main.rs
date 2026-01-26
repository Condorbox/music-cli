mod cli;
mod player;
mod models;
mod library;
mod utils;
mod ui;

use cli::{Cli, Commands};
use clap::Parser;

use player::audio;
use library::store::StoreManager;
use library::playlist;
use ui::terminal::TerminalUi;
use crate::player::controller;
use crate::ui::tui::TuiUi;
use crate::ui::Ui;
use crate::utils::APP_NAME;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let store = StoreManager::new()?;

    let mut ui = TerminalUi::new();

    match cli.command {
        Commands::Play { file } => {
            audio::play_file(file, &mut ui)?;
        }

        Commands::Path { directory } => {
            playlist::handle_set_path(
                directory.to_string_lossy().to_string(),
                &store,
                &mut ui
            )?;
        }

        Commands::Refresh => {
            playlist::handle_refresh(&store, &mut ui)?;
        }

        Commands::Playlist => {
            playlist::handle_playlist(&store, &mut ui)?;
        }

        Commands::List => {
            playlist::handle_list(&store, &mut ui)?;
        }

        Commands::Select { index } => {
            playlist::handle_select(index, &store, &mut ui)?;
        }

        Commands::Search { query } => {
            playlist::handle_search(query, &store, &mut ui)?;
        }

        Commands::Browse => {
            let mut ui = TuiUi::new();
            ui.init()?;

            let state = store.load()?;
            if state.library.is_empty() {
                ui.cleanup()?;
                ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
                return Ok(());
            }

            ui.set_songs(state.library);

            let result = controller::run_tui_player(&mut ui);

            ui.cleanup()?;

            result?;
        }
    }

    Ok(())
}