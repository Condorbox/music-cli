use anyhow::Result;

use crate::models::Song;
use crate::player::audio_player::AudioPlayer;
use crate::player::terminal_controller::TerminalController;
use crate::ui::Ui;

pub fn play_file(path: std::path::PathBuf, ui: &mut impl Ui) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("File not found: {}", path.display());
    }

    if !path.is_file() {
        anyhow::bail!("Path is not a file: {}", path.display());
    }

    let song = Song::from_path(&path);
    play_song(&song, ui)
}

pub fn play_song(song: &Song, ui: &mut impl Ui) -> Result<()> {
    let mut player = AudioPlayer::new()?;
    TerminalController::play_song(&mut player, song, ui)?;
    Ok(())
}

pub fn play_playlist(songs: Vec<Song>, ui: &mut impl Ui) -> Result<()> {
    let mut player = AudioPlayer::new()?;
    TerminalController::play_playlist(&mut player, songs, ui)?;
    Ok(())
}