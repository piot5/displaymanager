use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(name = "flux-cli", version = "1.0", about = "Unified Display/DDC Manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Display(DisplayArgs),
    Edid,
    Ddc(DdcArgs),
}

#[derive(Args, Debug)]
pub struct DisplayArgs {
    #[arg(short, long)] pub scan: bool,
    #[arg(long)] pub output: Option<String>,
    #[arg(long)] pub mode: Option<String>,
    #[arg(long)] pub pos: Option<String>,
    #[arg(long)] pub rotate: Option<String>,
    #[arg(long)] pub off: bool,
}

#[derive(Args, Debug)]
pub struct DdcArgs {
    #[arg(short, long, default_value_t = 0)] pub id: usize,
    #[command(subcommand)] pub action: DdcAction,
}

#[derive(Subcommand, Debug)]
pub enum DdcAction {
    List,
    Brightness { value: u32 },
    Contrast { value: u32 },
    Volume { value: u32 },
    Power { state: String },
    Input { source: String },
}