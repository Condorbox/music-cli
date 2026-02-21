use crate::core::models::Song;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// Result of a fuzzy search operation
#[derive(Debug, Clone)]
pub struct SearchResult<'a> {
    /// Original index in the full library
    pub index: usize,
    /// The matched song
    pub song: &'a Song,
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
    pub fn search<'a>(&self, library: &'a [Song], query: &str) -> Vec<SearchResult<'a>> {
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();

        let mut results: Vec<SearchResult> = library
            .iter()
            .enumerate()
            .filter_map(|(index, song)| {
                self.score_song(song, &query_lower).map(|score| SearchResult {
                    index,
                    song, 
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

        let combined_score = self.matcher.fuzzy_match(&song.search_key, query);

        // Extract the absolute maximum score across all fields and the combined search_key.
        [title_score, artist_score, album_score, combined_score]
            .into_iter()
            .flatten() // Automatically drops None values and unwraps Some(i64)
            .max()     // Grabs the highest score
    }

    /// Converts SearchResult to (index, Song) tuples by cloning
    pub fn search_result_to_song_index(&self, search_results: Vec<SearchResult<'_>>) -> Vec<(usize, Song)> {
        search_results
            .into_iter()
            .map(|result| (result.index, result.song.clone()))
            .collect()
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::Song;
    use std::path::PathBuf;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_song(title: &str, artist: Option<&str>, album: Option<&str>) -> Song {
        let artist_str = artist.map(str::to_owned);
        let album_str = album.map(str::to_owned);
        let search_key = format!(
            "{} {} {}",
            title,
            artist_str.as_deref().unwrap_or_default(),
            album_str.as_deref().unwrap_or_default()
        )
            .to_lowercase();

        Song {
            path: PathBuf::from(format!("{}.mp3", title)),
            title: title.to_owned(),
            artist: artist_str,
            album: album_str,
            track_number: None,
            duration: None,
            search_key,
        }
    }

    fn library() -> Vec<Song> {
        vec![
            make_song("Wish You Were Here", Some("Pink Floyd"), Some("Wish You Were Here")),
            make_song("Comfortably Numb", Some("Pink Floyd"), Some("The Wall")),
            make_song("Bohemian Rhapsody", Some("Queen"), Some("A Night at the Opera")),
            make_song("Under Pressure", Some("Queen"), Some("Hot Space")),
            make_song("Space Oddity", Some("David Bowie"), Some("Space Oddity")),
        ]
    }

    // ── Basic behaviour ───────────────────────────────────────────────────────

    #[test]
    fn empty_query_returns_empty_results() {
        let engine = SearchEngine::new();
        let library = &library();
        let results = engine.search(library, "");
        assert!(results.is_empty());
    }

    #[test]
    fn empty_library_returns_empty_results() {
        let engine = SearchEngine::new();
        let results = engine.search(&[], "Pink Floyd");
        assert!(results.is_empty());
    }

    #[test]
    fn no_match_returns_empty_results() {
        let engine = SearchEngine::new();
        let library = &library();
        let results = engine.search(library, "zxqwerty_no_match_xyz");
        assert!(results.is_empty());
    }

    // ── Field matching ────────────────────────────────────────────────────────

    #[test]
    fn exact_title_match_returns_correct_song() {
        let engine = SearchEngine::new();
        let lib = library();
        let results = engine.search(&lib, "Comfortably Numb");
        assert!(!results.is_empty());
        assert_eq!(results[0].song.title, "Comfortably Numb");
    }

    #[test]
    fn artist_match_returns_songs_by_that_artist() {
        let engine = SearchEngine::new();
        let lib = library();
        let results = engine.search(&lib, "Pink Floyd");
        // Both Pink Floyd songs should appear.
        let titles: Vec<&str> = results.iter().map(|r| r.song.title.as_str()).collect();
        assert!(titles.contains(&"Wish You Were Here") || titles.contains(&"Comfortably Numb"),
                "at least one Pink Floyd song must match");
    }

    #[test]
    fn album_match_returns_correct_song() {
        let engine = SearchEngine::new();
        let lib = library();
        // "The Wall" is an album name.
        let results = engine.search(&lib, "The Wall");
        assert!(!results.is_empty());
        let titles: Vec<&str> = results.iter().map(|r| r.song.title.as_str()).collect();
        assert!(titles.contains(&"Comfortably Numb"), "Comfortably Numb is on The Wall album");
    }

    // ── Ordering ──────────────────────────────────────────────────────────────

    #[test]
    fn results_are_sorted_best_score_first() {
        let engine = SearchEngine::new();
        let lib = library();
        let results = engine.search(&lib, "Queen");
        assert!(results.len() >= 2, "should return both Queen songs");
        // Scores must be non-increasing.
        for window in results.windows(2) {
            assert!(
                window[0].score >= window[1].score,
                "results must be sorted by score descending: {} >= {}",
                window[0].score, window[1].score
            );
        }
    }

    #[test]
    fn fuzzy_partial_match_returns_results() {
        let engine = SearchEngine::new();
        let lib = library();
        // "bohrap" is a fuzzy abbreviation of "Bohemian Rhapsody"
        let results = engine.search(&lib, "bohrap");
        assert!(!results.is_empty(), "fuzzy partial match should return results");
    }

    // ── Index correctness ─────────────────────────────────────────────────────

    #[test]
    fn result_index_matches_position_in_library() {
        let engine = SearchEngine::new();
        let lib = library();
        let results = engine.search(&lib, "Space Oddity");
        assert!(!results.is_empty());
        let result = &results[0];
        // Verify the reported index actually points to the matching song.
        assert_eq!(lib[result.index].title, result.song.title);
    }

    // ── search_result_to_song_index ───────────────────────────────────────────

    #[test]
    fn search_result_to_song_index_preserves_order_and_index() {
        let engine = SearchEngine::new();
        let lib = library();
        let raw = engine.search(&lib, "Queen");
        let indexed = engine.search_result_to_song_index(raw.clone());

        assert_eq!(raw.len(), indexed.len());
        for (raw_result, (idx, song)) in raw.iter().zip(indexed.iter()) {
            assert_eq!(raw_result.index, *idx, "index must be preserved");
            assert_eq!(raw_result.song.title, song.title, "song must be preserved");
        }
    }

    #[test]
    fn search_result_to_song_index_on_empty_input_returns_empty() {
        let engine = SearchEngine::new();
        let indexed = engine.search_result_to_song_index(vec![]);
        assert!(indexed.is_empty());
    }
}