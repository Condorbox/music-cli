use std::fmt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use lofty::probe::Probe;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AppConfig {
    pub root_path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Song {
    pub path: PathBuf,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<u32>,
    pub duration: Option<std::time::Duration>,
}

impl Song {
    pub fn from_path(path: &Path) -> Self {
        match Self::extract_metadata(path) {
            Ok(song) => song,
            Err(_) => Self::fallback(path),
        }
    }

    pub fn format_duration(&self) -> String {
        let seconds = self.duration.map(|d| d.as_secs()).unwrap_or(0);
        let mins = seconds / 60;
        let secs = seconds % 60;
        format!("{}:{:02}", mins, secs)
    }

    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.title.to_lowercase().contains(&q)
            || self.artist.as_ref().map_or(false, |a| a.to_lowercase().contains(&q))
            || self.album.as_ref().map_or(false, |a| a.to_lowercase().contains(&q))
    }

    fn extract_metadata(path: &Path) -> anyhow::Result<Self> {
        let tagged_file = Probe::open(path)?.read()?;
        let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

        Ok(Song {
            path: path.to_path_buf(),
            title: tag.and_then(|t| t.title().map(|s| s.into_owned()))
                .unwrap_or_else(|| Self::extract_filename(path)),
            artist: tag.and_then(|t| t.artist().map(|s| s.into_owned())),
            album: tag.and_then(|t| t.album().map(|s| s.into_owned())),
            track_number: tag.and_then(|t| t.track()),
            duration: Some(tagged_file.properties().duration()),
        })
    }

    fn fallback(path: &Path) -> Self {
        Song {
            path: path.to_path_buf(),
            title: Self::extract_filename(path),
            artist: None,
            album: None,
            track_number: None,
            duration: None,
        }
    }

    fn extract_filename(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string()
    }
}

impl fmt::Display for Song {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let duration_str = self.duration
            .map(|d| {
                let s = d.as_secs();
                format!("{}:{:02}", s / 60, s % 60)
            })
            .unwrap_or_else(|| "--:--".to_string());

        write!(
            f,
            "{} - {} [{}]",
            self.artist.as_deref().unwrap_or("Unknown Artist"),
            self.title,
            duration_str
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppState {
    pub config: AppConfig,
    pub library: Vec<Song>,
}