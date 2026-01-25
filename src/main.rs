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
    }

    Ok(())
}