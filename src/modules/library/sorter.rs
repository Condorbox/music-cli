use clap::builder::PossibleValue;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use crate::core::models::Song;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortField {
    /// Alphabetical by track title (default).
    #[default]
    Title,
    /// Alphabetical by the first credited artist.
    Artist,
    /// Alphabetical by album name; tracks without an album sort last.
    Album,
    /// Shortest to longest; tracks without duration sort last.
    Duration,
}

impl ValueEnum for SortField {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Title, Self::Artist, Self::Album, Self::Duration]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::Title    => Some(PossibleValue::new("title").help("Sort alphabetically by title (default)")),
            Self::Artist   => Some(PossibleValue::new("artist").help("Sort alphabetically by artist")),
            Self::Album    => Some(PossibleValue::new("album").help("Sort alphabetically by album")),
            Self::Duration => Some(PossibleValue::new("duration").help("Sort shortest to longest")),
        }
    }
}

impl SortField {
    /// Cycle to the next field: Title → Artist → Album → Duration → Title
    pub fn next(self) -> Self {
        match self {
            Self::Title    => Self::Artist,
            Self::Artist   => Self::Album,
            Self::Album    => Self::Duration,
            Self::Duration => Self::Title,
        }
    }
}

/// Return a sorted copy of `songs` according to `field`.
///
/// The original slice is never mutated — callers decide what to do with
/// the sorted view (print it, pass it to the TUI, etc.).
pub fn sort_songs(songs: &[Song], field: SortField) -> Vec<&Song> {
    let mut sorted: Vec<&Song> = songs.iter().collect();

    match field {
        SortField::Title => {
            sorted.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        }
        SortField::Artist => {
            sorted.sort_by(|a, b| {
                let a_artist = a.artists.first().map(|s| s.to_lowercase()).unwrap_or_default();
                let b_artist = b.artists.first().map(|s| s.to_lowercase()).unwrap_or_default();
                a_artist.cmp(&b_artist)
            });
        }
        SortField::Album => {
            sorted.sort_by(|a, b| {
                // Songs without an album float to the bottom.
                match (&a.album, &b.album) {
                    (None, None)       => std::cmp::Ordering::Equal,
                    (None, Some(_))    => std::cmp::Ordering::Greater,
                    (Some(_), None)    => std::cmp::Ordering::Less,
                    (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                }
            });
        }
        SortField::Duration => {
            sorted.sort_by(|a, b| {
                // Songs without a known duration float to the bottom.
                match (a.duration, b.duration) {
                    (None, None)       => std::cmp::Ordering::Equal,
                    (None, Some(_))    => std::cmp::Ordering::Greater,
                    (Some(_), None)    => std::cmp::Ordering::Less,
                    (Some(a), Some(b)) => a.cmp(&b),
                }
            });
        }
    }

    sorted
}