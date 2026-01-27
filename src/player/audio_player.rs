use std::fs::File;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, Sink};
use anyhow::{Result, Context};

use crate::models::Song;

pub struct AudioPlayer {
    _stream: OutputStream,
    sink: Sink,
    current_song: Option<Song>,
    is_paused: bool,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;

        Ok(Self {
            _stream,
            sink,
            current_song: None,
            is_paused: false,
        })
    }

    /// Play a song, replacing any current playback
    pub fn play(&mut self, song: &Song) -> Result<()> {
        self.sink.stop();

        let file = File::open(&song.path)?;
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Failed to decode audio file: {}", song.path.display()))?;

        self.sink.append(source);
        self.current_song = Some(song.clone());
        self.is_paused = false;

        Ok(())
    }

    /// Stop playback and clear current song
    pub fn stop(&mut self) {
        self.sink.stop();
        self.current_song = None;
        self.is_paused = false;
    }

    /// Pause playback (if playing)
    pub fn pause(&mut self) {
        if self.current_song.is_some() && !self.is_paused {
            self.sink.pause();
            self.is_paused = true;
        }
    }

    /// Resume playback (if paused)
    pub fn resume(&mut self) {
        if self.current_song.is_some() && self.is_paused {
            self.sink.play();
            self.is_paused = false;
        }
    }

    /// Toggle between paused and playing
    pub fn toggle_pause(&mut self) {
        if self.is_paused {
            self.resume();
        } else {
            self.pause();
        }
    }

    /// Check if currently paused
    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    /// Check if actively playing (has song loaded and not finished)
    pub fn is_playing(&self) -> bool {
        self.current_song.is_some() && !self.sink.empty()
    }

    /// Check if the current track has finished playing
    pub fn has_finished(&self) -> bool {
        self.current_song.is_some() && self.sink.empty()
    }

    /// Get the currently loaded song (playing, paused, or finished)
    pub fn current_song(&self) -> Option<&Song> {
        self.current_song.as_ref()
    }

    /// Get the playback volume (0.0 to 1.0)
    pub fn volume(&self) -> f32 {
        self.sink.volume()
    }

    /// Set the playback volume (0.0 to 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.sink.set_volume(volume.clamp(0.0, 1.0));
    }
}

// To avoid leaks
impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.sink.stop();
    }
}