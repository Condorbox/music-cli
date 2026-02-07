use crate::core::models::Song;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// Result of a fuzzy search operation
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Original index in the full library
    pub index: usize,
    /// The matched song
    pub song: Song,
    /// Match score (higher is better)
    pub score: i64,
}

/// Search engine for finding songs with fuzzy matching
pub struct SearchEngine {
    matcher: SkimMatcherV2,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Perform a fuzzy search across the library
    ///
    /// Returns results sorted by relevance (best matches first)
    ///
    /// # Arguments
    /// * `library` - The full song library to search
    /// * `query` - The search query string
    ///
    /// # Returns
    /// Vector of SearchResult, sorted by score (descending)
    pub fn search(&self, library: &[Song], query: &str) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<SearchResult> = library
            .iter()
            .enumerate()
            .filter_map(|(index, song)| {
                self.score_song(song, query).map(|score| SearchResult {
                    index,
                    song: song.clone(),
                    score,
                })
            })
            .collect();

        // Sort by score descending (best matches first)
        results.sort_by(|a, b| b.score.cmp(&a.score));

        results
    }

    /// Calculate a match score for a single song
    ///
    /// Searches across title, artist, and album fields
    /// Returns None if no match found
    fn score_song(&self, song: &Song, query: &str) -> Option<i64> {
        // Try matching against individual fields first (higher weight)
        let title_score = self.matcher.fuzzy_match(&song.title, query);
        let artist_score = song.artist.as_ref()
            .and_then(|a| self.matcher.fuzzy_match(a, query));
        let album_score = song.album.as_ref()
            .and_then(|a| self.matcher.fuzzy_match(a, query));

        // Take the best individual field match
        let best_field_score = [title_score, artist_score, album_score]
            .iter()
            .filter_map(|&s| s)
            .max();

        // Also try matching against combined text (fallback)
        let combined_text = format!(
            "{} {} {}",
            song.title,
            song.artist.as_deref().unwrap_or(""),
            song.album.as_deref().unwrap_or("")
        );
        let combined_score = self.matcher.fuzzy_match(&combined_text, query);

        // Return the best score we found
        best_field_score.or(combined_score)
    }

    /// Converts SearchResult to (index, Song) tuples
    pub fn search_result_to_song_index(&self, search_results: Vec<SearchResult>) -> Vec<(usize, Song)> {
        search_results
            .into_iter()
            .map(|result| (result.index, result.song))
            .collect()
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
