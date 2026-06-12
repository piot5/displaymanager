mod cli_ddc;
pub mod display;

pub use cli_ddc::{DdcAction, DdcArgs};
pub use display::{DisplaySubcommand, SetArgs};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "displaymanager_cli", version = "1.0", about = "Unified Display/DDC Manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Display topology: scan, info, set
    #[command(subcommand)]
    Display(DisplaySubcommand),
    /// DDC/CI brightness, contrast, input, power (df_ddc)
    Ddc(DdcArgs),
}
