use std::time::Duration;

/// Represents the current state of song playback progress
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackProgress {
    elapsed: Duration,
    total: Duration,
}

impl PlaybackProgress {
    /// Creates a new PlaybackProgress instance
    ///
    /// # Arguments
    /// * `elapsed` - The current elapsed time
    /// * `total` - The total duration of the song
    ///
    /// # Returns
    /// * `Some(PlaybackProgress)` if total duration is valid (> 0)
    /// * `None` if total duration is zero or would cause invalid state
    pub fn new(elapsed: Duration, total: Duration) -> Option<Self> {
        if total.as_secs() == 0 {
            return None;
        }

        Some(Self {
            elapsed: elapsed.min(total), // Clamp elapsed to total
            total,
        })
    }

    /// Returns the ratio of progress (0.0 to 1.0)
    pub fn ratio(&self) -> f64 {
        (self.elapsed.as_secs_f64() / self.total.as_secs_f64()).clamp(0.0, 1.0)
    }

    /// Returns the elapsed duration
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns the total duration
    pub fn total(&self) -> Duration {
        self.total
    }
}