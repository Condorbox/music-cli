use crate::core::traits::PlaybackBackend;
use crate::core::models::Song;
use anyhow::{Result, Context};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

pub struct RodioBackend {
    device_sink: MixerDeviceSink,
    player: Player,
    current_song: Option<Song>,
}

impl RodioBackend {
    pub fn new() -> Result<Self> {
        let mut device_sink = DeviceSinkBuilder::open_default_sink()
            .context("Failed to open default audio output device")?;
        device_sink.log_on_drop(false);
        let player = Player::connect_new(device_sink.mixer());

        Ok(Self {
            device_sink,
            player,
            current_song: None,
        })
    }
}

impl PlaybackBackend for RodioBackend {
    fn play(&mut self, song: &Song) -> Result<()> {
        let volume = self.player.volume();
        self.player = Player::connect_new(self.device_sink.mixer());
        self.player.set_volume(volume);

        let file = File::open(&song.path)?;
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Failed to decode audio file: {}", song.path.display()))?;

        self.player.append(source);
        self.current_song = Some(song.clone());
        self.player.play();

        Ok(())
    }

    fn stop(&mut self) {
        self.player.stop();
        self.current_song = None;
    }

    fn pause(&mut self) {
        if self.current_song.is_some() {
            self.player.pause();
        }
    }

    fn resume(&mut self) {
        if self.current_song.is_some() {
            self.player.play();
        }
    }

    fn is_playing(&self) -> bool {
        self.current_song.is_some() && !self.player.empty()
    }

    fn is_paused(&self) -> bool {
        self.current_song.is_some() && self.player.is_paused()
    }

    fn has_finished(&self) -> bool {
        self.current_song.is_some() && self.player.empty()
    }

    fn set_volume(&mut self, volume: f32) {
        self.player.set_volume(volume.clamp(0.0, 1.0));
    }

    fn position(&self) -> Duration {
        if self.current_song.is_some() {
            self.player.get_pos()
        } else {
            Duration::ZERO
        }
    }
}
