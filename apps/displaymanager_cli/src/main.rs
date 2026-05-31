// apps/displaymanager_cli/src/main.rs
mod cli;
mod logic;
mod synth; // Register the synthesis module at the crate root
mod utils;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Display(args) => logic::handle_display(args).await?,
        Commands::Edid => logic::handle_edid()?,
        Commands::Ddc(args) => logic::handle_ddc(args)?,
    }
    Ok(())
}