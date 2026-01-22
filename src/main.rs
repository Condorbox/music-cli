mod cli;

use cli::{Cli, Commands};

use clap::Parser;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Play { file } => {
            play_file(file)?;
        }
    }

    Ok(())
}

fn play_file(path: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let file = File::open(path)?;
    let source = Decoder::new(BufReader::new(file))?;

    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}
