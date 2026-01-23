mod cli;
mod player;
mod models;
mod library;

use cli::{Cli, Commands};
use clap::Parser;
use player::audio;
use library::store::StoreManager;
use library::playlist;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let store = StoreManager::new()?;

    match cli.command {
        Commands::Play { file } => {
            audio::play_file(file)?;
        }

        Commands::Path { directory } => {
            playlist::handle_set_path(
                directory.to_string_lossy().to_string(),
                &store
            )?;
        }

        Commands::Refresh => {
            playlist::handle_refresh(&store)?;
        }

        Commands::Playlist => {
            playlist::handle_playlist(&store)?;
        }

    }

    Ok(())
}