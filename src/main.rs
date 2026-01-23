mod cli;
mod player;

use cli::{Cli, Commands};
use clap::Parser;
use player::audio;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Play { file } => {
            audio::play_file(file)?;
        }
    }

    Ok(())
}