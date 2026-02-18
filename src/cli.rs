use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use clap::builder::PossibleValue;
use crate::core::models::RepeatMode;
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

    /// Search for songs in your library
    Search {
        /// Search query (searches title, artist, and album)
        query: String,
    },

    /// Browse and play songs with interactive TUI
    Browse,

    /// Set volume between 0 and 100 (or show current if no argument)
    Volume {
        /// Volume level (0 - 100). If omitted, shows current volume
        #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
        volume: Option<u8>,
    },

    /// Toggle shuffle mode for playlist playback
    Shuffle {
        /// Explicitly set shuffle state (true/false). If omitted, toggles current state
        #[arg(value_parser = clap::value_parser!(bool))]
        enabled: Option<bool>,
    },

    /// Set repeat mode (off/all/one). Cycles to the next mode if no argument given
    Loop {
        /// Repeat mode: off, all, one. If omitted, cycles to the next mode
        #[arg(value_enum)]
        mode: Option<RepeatMode>,
    },
}

impl ValueEnum for RepeatMode {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Off, Self::All, Self::One]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::Off => Some(PossibleValue::new("off").help("Stop at the end of the playlist")),
            Self::All => Some(PossibleValue::new("all").help("Loop the entire playlist")),
            Self::One => Some(PossibleValue::new("one").help("Repeat the current song")),
        }
    }
}