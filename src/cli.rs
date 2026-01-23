use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "music-cli")]
#[command(about = "Lightweight terminal music player", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Play a music file
    Play {
        /// Path to the audio file path
        file: PathBuf,
    },

    /// Set the root music directory path
    Path {
        /// Path to the music directory
        directory: PathBuf,
    },

    /// Refresh the music library from the configured path
    Refresh,

    /// Play songs from the library or a custom directory
    Playlist,

}
