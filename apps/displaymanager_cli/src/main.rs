//! Display Manager CLI Application
//!
//! Structured into logical subcommands:
//!   display scan     — scan all monitors, EDID report, JSON export
//!   display info     — single-monitor details
//!   display set      — topology: position, resolution, rotation, HDR, scale, primary
//!   ddc              — brightness, contrast, volume, input, power, color-gains

mod cli;
mod ddc;
mod info;
mod set;

use clap::Parser;
use cli::{Cli, Commands, DisplaySubcommand};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Display(sub) => match sub {
            DisplaySubcommand::Scan(args) => {
                // Combined scan + edid report: writes edid_dump.txt automatically
                info::write_edid_report("edid_dump.txt")?;
                if let Some(ref path) = args.edid_json {
                    info::write_edid_json(path)?;
                } else if args.json {
                    info::scan_json()?;
                } else {
                    info::scan()?;
                }
            }
            DisplaySubcommand::Info(args) => {
                if args.json {
                    info::monitor_info_json(&args.output)?;
                } else {
                    info::monitor_info(&args.output)?;
                }
            }
            DisplaySubcommand::Set(args) => {
                set::apply(&args.output, &args).await?;
            }
        },
        // `edid` removed: merged into `display scan`
        Commands::Ddc(args) => ddc::run(args)?,
    }

    Ok(())
}
