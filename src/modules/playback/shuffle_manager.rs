use rand::seq::SliceRandom;

/// Manages shuffle state and provides smart randomization without repetition
///
/// Uses a "shuffle queue" approach:
/// - Creates a randomized queue of all song indices
/// - Plays through the queue in order
/// - When queue is exhausted, reshuffles and starts over
/// - This guarantees no repetition within a single pass through the playlist
#[derive(Debug, Clone)]
pub struct ShuffleManager {
    /// Whether shuffle is currently enabled
    enabled: bool,

    /// Queue of indices to play (in shuffled order)
    /// When empty, needs to be regenerated
    shuffle_queue: Vec<usize>,

    /// Current position in the shuffle queue
    queue_position: usize,

    /// Total size of the playlist (for regenerating queue)
    playlist_size: usize,
}

impl ShuffleManager {
    /// Create a new shuffle manager
    pub fn new() -> Self {
        Self {
            enabled: false,
            shuffle_queue: Vec::new(),
            queue_position: 0,
            playlist_size: 0,
        }
    }

    /// Toggle shuffle on/off
    pub fn toggle(&mut self) -> bool {
        self.set_enabled(!self.enabled);
        self.enabled
    }

    /// Set shuffle state explicitly
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }

        self.enabled = enabled;
        if !self.enabled {
            self.shuffle_queue.clear();
            self.queue_position = 0;
        }
    }

    /// Check if shuffle is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Initialize shuffle for a new playlist
    ///
    /// Call this when:
    /// - Starting a new playlist
    /// - Playlist is modified
    /// - Shuffle is toggled on
    pub fn initialize(&mut self, playlist_size: usize, current_index: Option<usize>) {
        self.playlist_size = playlist_size;
        if self.enabled && playlist_size > 0 {
            self.generate_shuffle_queue(current_index);
        }
    }

    /// Get the next index to play
    ///
    /// # Arguments
    /// * `current_index` - The current song
    /// * `loop_playlist` - If true, reshuffles and continues when queue ends.
    ///                     If false, returns None at end of queue.
    pub fn next_index(&mut self, current_index: Option<usize>, loop_playlist: bool) -> Option<usize> {
        if !self.enabled {
            return current_index.and_then(|idx| {
                if idx + 1 < self.playlist_size { Some(idx + 1) }
                else if loop_playlist { Some(0) } // Handle loop in sequential too
                else { None }
            });
        }

        if self.shuffle_queue.is_empty() {
            self.generate_shuffle_queue(current_index);
        }

        let next_pos = self.queue_position + 1;

        // Check if we reached the end of the shuffled queue
        if next_pos >= self.shuffle_queue.len() {
            if !loop_playlist {
                return None; // Stop playback
            }

            // Capture the last song played to avoid immediate repeat
            let last_played = self.shuffle_queue.last().copied();

            // Reshuffle
            self.generate_shuffle_queue(None);

            // EDGE CASE FIX: Ensure the first song of new queue isn't the same as the last song
            // (Only matters if playlist size > 1)
            if self.playlist_size > 1 {
                if let (Some(last), Some(first)) = (last_played, self.shuffle_queue.first()) {
                    if last == *first {
                        // Swap first element with the last element to break the repeat
                        let last_idx = self.shuffle_queue.len() - 1;
                        self.shuffle_queue.swap(0, last_idx);
                    }
                }
            }

            // Reset position for new queue
            self.queue_position = 0;
        } else {
            self.queue_position = next_pos;
        }

        self.shuffle_queue.get(self.queue_position).copied()
    }

    /// Get the previous index to play
    ///
    /// # Arguments
    /// * `current_index` - The current song index
    ///
    /// # Returns
    /// * `Some(usize)` - Previous index to play
    /// * `None` - Already at start
    pub fn previous_index(&mut self, current_index: Option<usize>) -> Option<usize> {
        if !self.enabled {
            return current_index.and_then(|idx| if idx > 0 { Some(idx - 1) } else { None });
        }

        if self.queue_position > 0 {
            self.queue_position -= 1;
            self.shuffle_queue.get(self.queue_position).copied()
        } else {
            // Note: Cannot go back to previous shuffle epoch without a history stack
            None
        }
    }

    /// Generate a new shuffle queue
    ///
    /// Creates a randomized list of indices, optionally ensuring
    /// the current song is first (to avoid jarring transitions)
    fn generate_shuffle_queue(&mut self, force_first: Option<usize>) {
        if self.playlist_size == 0 {
            self.shuffle_queue.clear();
            return;
        }

        let mut indices: Vec<usize> = (0..self.playlist_size).collect();
        let mut rng = rand::rng(); // rand 0.9+ syntax
        indices.shuffle(&mut rng);

        // Logic: If a specific song MUST be first (because it's currently playing
        // when we enabled shuffle), swap it to position 0.
        if let Some(first) = force_first {
            if let Some(pos) = indices.iter().position(|&i| i == first) {
                indices.swap(0, pos);
            }
        }

        self.shuffle_queue = indices;
        self.queue_position = 0;
    }

    /// Update playlist size (call when playlist changes)
    pub fn update_playlist_size(&mut self, new_size: usize) {
        if self.playlist_size != new_size {
            self.playlist_size = new_size;

            // Regenerate queue if shuffle is enabled
            if self.enabled {
                self.generate_shuffle_queue(None);
            }
        }
    }

    /// Get current position in shuffle queue (for debugging/display)
    pub fn queue_position(&self) -> usize {
        self.queue_position
    }

    /// Get remaining songs in current shuffle pass
    pub fn remaining_in_pass(&self) -> usize {
        if self.shuffle_queue.is_empty() {
            0
        } else {
            self.shuffle_queue
                .len()
                .saturating_sub(self.queue_position + 1)
        }
    }
}

impl Default for ShuffleManager {
    fn default() -> Self {
        Self::new()
    }
}
