use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::utils::APP_NAME;

#[derive(Parser)]
#[command(name = APP_NAME)]
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

    /// Play songs from the library from the configured path
    Playlist,

    /// List song form the library from the configured path
    List,

    /// Select a song from your library by index
    Select {
        /// Song index
        index: usize,
    },
}
