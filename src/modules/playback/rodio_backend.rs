use crate::core::traits::PlaybackBackend;
use crate::core::models::Song;
use anyhow::{Result, Context};
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

pub struct RodioBackend {
    sink: Sink,
    current_song: Option<Song>,
    is_paused: bool,
}

impl RodioBackend {
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) = OutputStream::try_default()?;

        // This keeps the audio engine running globally for the life of the program
        // without binding it to this struct.
        // If we simply dropped it, sound would stop.
        std::mem::forget(stream);

        let sink = Sink::try_new(&stream_handle)?;

        Ok(Self {
            sink,
            current_song: None,
            is_paused: false,
        })
    }
}

impl PlaybackBackend for RodioBackend {
    fn play(&mut self, song: &Song) -> Result<()> {
        self.sink.stop();

        let file = File::open(&song.path)?;
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Failed to decode audio file: {}", song.path.display()))?;

        self.sink.append(source);
        self.current_song = Some(song.clone());
        self.is_paused = false;

        self.sink.play();

        Ok(())
    }

    fn stop(&mut self) {
        self.sink.stop();
        self.current_song = None;
        self.is_paused = false;
    }

    fn pause(&mut self) {
        if self.current_song.is_some() && !self.is_paused {
            self.sink.pause();
            self.is_paused = true;
        }
    }

    fn resume(&mut self) {
        if self.current_song.is_some() && self.is_paused {
            self.sink.play();
            self.is_paused = false;
        }
    }

    fn is_playing(&self) -> bool {
        self.current_song.is_some() && !self.sink.empty()
    }

    fn is_paused(&self) -> bool {
        self.is_paused
    }

    fn has_finished(&self) -> bool {
        self.current_song.is_some() && self.sink.empty()
    }

    fn current_song(&self) -> Option<&Song> {
        self.current_song.as_ref()
    }

    fn set_volume(&mut self, volume: f32) {
        self.sink.set_volume(volume.clamp(0.0, 1.0));
    }

    fn volume(&self) -> f32 {
        self.sink.volume()
    }
}

// To avoid leaks
impl Drop for RodioBackend {
    fn drop(&mut self) {
        self.sink.stop();
    }
}
