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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Build an initialized, enabled ShuffleManager ready to use.
    fn enabled_manager(size: usize) -> ShuffleManager {
        let mut m = ShuffleManager::new();
        m.set_enabled(true);
        m.initialize(size, None);
        m
    }

    // ── Construction ──────────────────────────────────────────────────────────

    #[test]
    fn new_is_disabled_with_empty_state() {
        let m = ShuffleManager::new();
        assert!(!m.is_enabled());
        assert_eq!(m.queue_position(), 0);
        assert_eq!(m.remaining_in_pass(), 0);
    }

    // ── toggle / set_enabled ──────────────────────────────────────────────────

    #[test]
    fn toggle_flips_enabled_and_returns_new_state() {
        let mut m = ShuffleManager::new();
        assert!(m.toggle());  // false → true
        assert!(m.is_enabled());
        assert!(!m.toggle()); // true → false
        assert!(!m.is_enabled());
    }

    #[test]
    fn set_enabled_same_value_is_noop() {
        let mut m = ShuffleManager::new();
        // Already disabled; calling set_enabled(false) should not clear or change anything.
        m.set_enabled(false);
        assert!(!m.is_enabled());

        m.set_enabled(true);
        m.initialize(5, None);
        let pos_before = m.queue_position();

        m.set_enabled(true); // no-op
        assert_eq!(m.queue_position(), pos_before, "position must not change on no-op");
    }

    #[test]
    fn set_enabled_false_clears_queue_and_position() {
        let mut m = enabled_manager(5);
        // Advance so queue_position is non-zero.
        m.next_index(Some(0), true);

        m.set_enabled(false);
        assert!(!m.is_enabled());
        assert_eq!(m.queue_position(), 0);
        assert_eq!(m.remaining_in_pass(), 0);
    }

    // ── initialize ────────────────────────────────────────────────────────────

    #[test]
    fn initialize_while_disabled_does_nothing() {
        let mut m = ShuffleManager::new();
        m.initialize(10, None);
        assert_eq!(m.remaining_in_pass(), 0, "disabled manager should have empty queue");
    }

    #[test]
    fn initialize_builds_queue_of_correct_size() {
        let m = enabled_manager(8);
        // remaining = len - (position+1); position starts at 0 → remaining = 7
        assert_eq!(m.remaining_in_pass(), 7);
    }

    #[test]
    fn initialize_queue_contains_all_indices_exactly_once() {
        const SIZE: usize = 12;
        let mut m = enabled_manager(SIZE);

        // Drain the whole queue and collect every index.
        let mut seen = Vec::new();
        let first = m.queue_position(); // already at position 0 after initialize
        seen.push(m.shuffle_queue[first]);
        while m.remaining_in_pass() > 0 {
            if let Some(idx) = m.next_index(seen.last().copied(), false) {
                seen.push(idx);
            }
        }

        seen.sort_unstable();
        let expected: Vec<usize> = (0..SIZE).collect();
        assert_eq!(seen, expected, "every index must appear exactly once per pass");
    }

    // ── next_index — shuffle DISABLED ─────────────────────────────────────────

    #[test]
    fn next_sequential_advances_by_one() {
        let mut m = ShuffleManager::new();
        m.playlist_size = 5;
        assert_eq!(m.next_index(Some(2), false), Some(3));
        assert_eq!(m.next_index(Some(0), false), Some(1));
    }

    #[test]
    fn next_sequential_no_loop_returns_none_at_end() {
        let mut m = ShuffleManager::new();
        m.playlist_size = 3;
        assert_eq!(m.next_index(Some(2), false), None);
    }

    #[test]
    fn next_sequential_with_loop_wraps_to_zero() {
        let mut m = ShuffleManager::new();
        m.playlist_size = 3;
        assert_eq!(m.next_index(Some(2), true), Some(0));
    }

    #[test]
    fn next_sequential_none_current_returns_none() {
        let mut m = ShuffleManager::new();
        m.playlist_size = 5;
        assert_eq!(m.next_index(None, true), None);
    }

    // ── previous_index — shuffle DISABLED ────────────────────────────────────

    #[test]
    fn prev_sequential_decrements_by_one() {
        let mut m = ShuffleManager::new();
        assert_eq!(m.previous_index(Some(3)), Some(2));
        assert_eq!(m.previous_index(Some(1)), Some(0));
    }

    #[test]
    fn prev_sequential_at_zero_returns_none() {
        let mut m = ShuffleManager::new();
        assert_eq!(m.previous_index(Some(0)), None);
    }

    #[test]
    fn prev_sequential_none_current_returns_none() {
        let mut m = ShuffleManager::new();
        assert_eq!(m.previous_index(None), None);
    }

    // ── next_index — shuffle ENABLED ─────────────────────────────────────────

    #[test]
    fn next_shuffle_advances_through_queue() {
        let mut m = enabled_manager(5);
        let mut visited = vec![m.shuffle_queue[0]]; // the current "playing" index
        for _ in 0..4 {
            let next = m.next_index(visited.last().copied(), false);
            assert!(next.is_some(), "should have a next song within the pass");
            visited.push(next.unwrap());
        }
        assert_eq!(visited.len(), 5);
    }

    #[test]
    fn next_shuffle_no_loop_returns_none_when_queue_exhausted() {
        let mut m = enabled_manager(3);
        // Drain all 3 positions.
        let mut last = m.shuffle_queue[0];
        m.next_index(Some(last), false); // pos 1
        last = m.shuffle_queue[1];
        m.next_index(Some(last), false); // pos 2
        last = m.shuffle_queue[2];

        // Next call should see end of queue with loop=false.
        let result = m.next_index(Some(last), false);
        assert_eq!(result, None, "should stop at end when loop=false");
    }

    #[test]
    fn next_shuffle_with_loop_reshuffles_and_continues() {
        let mut m = enabled_manager(3);
        // Exhaust the first pass.
        let mut last = m.shuffle_queue[0];
        m.next_index(Some(last), true);
        last = m.shuffle_queue[1];
        m.next_index(Some(last), true);
        last = m.shuffle_queue[2];

        // This call hits end-of-queue but loop=true → reshuffle.
        let result = m.next_index(Some(last), true);
        assert!(result.is_some(), "should produce a song after reshuffle");
        let idx = result.unwrap();
        assert!(idx < 3, "index must be within playlist bounds");
    }

    #[test]
    fn next_shuffle_no_immediate_repeat_after_reshuffle() {
        // Run many reshuffles and verify the first song of a new pass never equals
        // the last song of the previous pass (only guaranteed when size > 1).
        let size = 4;
        let mut m = enabled_manager(size);

        for _ in 0..20 {
            // Drain to the last position.
            let mut last = m.shuffle_queue[0];
            for _ in 1..size {
                last = m.next_index(Some(last), true).unwrap();
            }
            let last_of_pass = last;
            // This triggers a reshuffle.
            let first_of_new_pass = m.next_index(Some(last_of_pass), true).unwrap();
            assert_ne!(
                last_of_pass, first_of_new_pass,
                "first song of new shuffle pass must not equal last song of previous pass"
            );
        }
    }

    // ── previous_index — shuffle ENABLED ─────────────────────────────────────

    #[test]
    fn prev_shuffle_walks_back_through_history() {
        let mut m = enabled_manager(5);
        let first = m.shuffle_queue[0];
        let second = m.next_index(Some(first), false).unwrap();

        // Going back should return the first.
        let returned = m.previous_index(Some(second));
        assert_eq!(returned, Some(first));
    }

    #[test]
    fn prev_shuffle_at_start_of_queue_returns_none() {
        let mut m = enabled_manager(5);
        // queue_position is 0 right after initialize.
        assert_eq!(m.previous_index(Some(m.shuffle_queue[0])), None);
    }

    // ── remaining_in_pass ─────────────────────────────────────────────────────

    #[test]
    fn remaining_in_pass_decrements_on_each_advance() {
        let size: usize = 6;
        let mut m = enabled_manager(size);
        assert_eq!(m.remaining_in_pass(), size - 1);

        let mut last = m.shuffle_queue[0];
        for expected_remaining in (0..size - 1).rev() {
            last = m.next_index(Some(last), false).unwrap();
            assert_eq!(m.remaining_in_pass(), expected_remaining);
        }
    }

    // ── update_playlist_size ──────────────────────────────────────────────────

    #[test]
    fn update_playlist_size_noop_when_same() {
        let mut m = enabled_manager(5);
        let pos_before = m.queue_position();
        m.update_playlist_size(5);
        assert_eq!(m.queue_position(), pos_before, "must not regenerate when size unchanged");
    }

    #[test]
    fn update_playlist_size_regenerates_queue_when_size_changes() {
        let mut m = enabled_manager(5);
        m.update_playlist_size(10);
        // After resize, position resets and queue fits new size.
        assert_eq!(m.queue_position(), 0);
        assert_eq!(m.remaining_in_pass(), 9, "new queue should have 10 entries");
    }
}