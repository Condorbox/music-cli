mod cli;
mod player;
mod models;
mod library;
mod utils;
mod ui;

use cli::{Cli, Commands};
use clap::Parser;
use player::audio;
use library::store::StoreManager;
use library::playlist;
use ui::terminal::TerminalUi;
use crate::ui::tui::TuiUi;
use crate::ui::Ui;
use crate::utils::APP_NAME;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let store = StoreManager::new()?;

    let mut ui = TerminalUi::new();

    match cli.command {
        Commands::Play { file } => {
            audio::play_file(file, &mut ui)?;
        }

        Commands::Path { directory } => {
            playlist::handle_set_path(
                directory.to_string_lossy().to_string(),
                &store,
                &mut ui
            )?;
        }

        Commands::Refresh => {
            playlist::handle_refresh(&store, &mut ui)?;
        }

        Commands::Playlist => {
            playlist::handle_playlist(&store, &mut ui)?;
        }

        Commands::List => {
            playlist::handle_list(&store, &mut ui)?;
        }

        Commands::Select { index } => {
            playlist::handle_select(index, &store, &mut ui)?;
        }

        Commands::Search { query } => {
            playlist::handle_search(query, &store, &mut ui)?;
        }

        Commands::Browse => {
            let mut ui = TuiUi::new();
            ui.init()?;

            let state = store.load()?;
            if state.library.is_empty() {
                ui.cleanup()?;
                ui.print_error(&format!("Library is empty. Run '{} refresh' first.", APP_NAME));
                return Ok(());
            }

            ui.set_songs(state.library);

            // Main TUI loop
            loop {
                ui.render()?;

                if let Some(event) = ui.handle_input(std::time::Duration::from_millis(100))? {
                    match event {
                        ui::tui::TuiEvent::Quit => break,
                        ui::tui::TuiEvent::PlaySelected => {
                            if let Some(song) = ui.get_selected_song() {
                                let song = song.clone();
                                ui.cleanup()?;
                                audio::play_song(&song, &mut ui)?;
                                ui.init()?;
                            }
                        }
                        _ => {}
                    }
                }
            }

            ui.cleanup()?;

        }
    }

    Ok(())
}