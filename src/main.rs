mod cli;
mod cli_handlers;
mod core;
mod application;
mod modules;
mod utils;

use cli::{Cli, Commands};
use clap::Parser;
use anyhow::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Route commands to their handlers
    match cli.command {
        Commands::Browse => cli_handlers::handle_browse(),
        Commands::Play { file } => cli_handlers::handle_play(file),
        Commands::Path { directory } => cli_handlers::handle_path(directory),
        Commands::Refresh => cli_handlers::handle_refresh(),
        Commands::Playlist => cli_handlers::handle_playlist(),
        Commands::List => cli_handlers::handle_list(),
        Commands::Select { index } => cli_handlers::handle_select(index),
        Commands::Search { query } => cli_handlers::handle_search(query),
        Commands::Volume { volume } => cli_handlers::handle_volume(volume),
        Commands::Shuffle { enabled } => cli_handlers::handle_shuffle(enabled),
        Commands::Loop { mode } => cli_handlers::handle_loop(mode),
    }
}