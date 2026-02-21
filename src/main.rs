mod cli;
mod cli_handlers;
mod core;
mod application;
mod modules;
mod utils;

use cli::Cli;
use clap::Parser;
use anyhow::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli_handlers::from_cli(cli.command).execute()
}
