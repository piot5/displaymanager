use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct DdcArgs {
    #[arg(short, long, default_value_t = 0, help = "DDC monitor index from list")]
    pub id: usize,
    /// Output DDC capabilities as JSON (with "list" command)
    #[arg(short, long)]
    pub json: bool,
    #[command(subcommand)]
    pub action: DdcAction,
}

#[derive(Subcommand, Debug)]
pub enum DdcAction {
    List,
    Brightness {
        value: u32,
    },
    Contrast {
        value: u32,
    },
    Volume {
        value: u32,
    },
    Power {
        state: String,
    },
    Input {
        source: String,
    },
    /// Set color gains: red, green, blue values (0..100 each)
    ColorGains {
        red: u32,
        green: u32,
        blue: u32,
    },
}
