//! Standalone DDC/CI monitor management CLI tool.
//!
//! Provides a subcommand interface for listing monitors and controlling
//! DDC/CI features like brightness, contrast, volume, input, and power.
//!
//! ## Usage
//!
//! ```text
//! ddc_mgr [OPTIONS] <COMMAND>
//!
//! Commands:
//!   list                List all connected DDC-capable monitors
//!   brightness <VALUE>  Set brightness (0-100)
//!   contrast <VALUE>    Set contrast (0-100)
//!   volume <VALUE>      Set audio volume (0-100)
//!   power <STATE>       Set power state (on/off)
//!   input <SOURCE>      Switch input source (hdmi1, hdmi2, dp1, dp2)
//! ```

use clap::{Parser, Subcommand};
use df_ddc::ddc_trait::DisplayDevice;
use df_ddc::ddc_types::{InputSource, PowerState, VcpCode};
use df_ddc::error::DdcError;
use df_ddc::list_monitors;
use std::process;

/// CLI tool to manage DDC/CI capable monitors.
#[derive(Parser)]
#[command(
    name = "monmgr",
    version = "1.0",
    about = "DDC/CI Monitor Manager",
    long_about = "Standalone utility for querying and controlling DDC/CI-capable monitors.\n\
                  Supports brightness, contrast, volume, input source, and power state."
)]
struct Cli {
    /// Select monitor by index (default: 0)
    #[arg(short, long, default_value_t = 0, global = true)]
    id: usize,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all connected DDC-capable monitors with their indices
    List,
    /// Set brightness (0-100)
    Brightness { value: u32 },
    /// Set contrast (0-100)
    Contrast { value: u32 },
    /// Set audio volume (0-100)
    Volume { value: u32 },
    /// Set power state (on/off)
    Power { state: String },
    /// Switch input source (hdmi1, hdmi2, dp1, dp2)
    Input { source: String },
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();
    let monitors = list_monitors();

    if monitors.is_empty() {
        eprintln!("Error: No DDC/CI capable monitors found.");
        std::process::exit(1);
    }

    // Select the monitor based on the index provided by the user
    let display = match monitors.get(cli.id) {
        Some(d) => d,
        None => {
            eprintln!(
                "Error: Monitor index {} not found. Available indices: 0-{}",
                cli.id,
                monitors.len() - 1
            );
            std::process::exit(1);
        }
    };

    let result = match &cli.command {
        Commands::List => {
            for (i, m) in monitors.iter().enumerate() {
                println!("[{}] {}", i, m.info);
            }
            Ok(())
        }
        Commands::Brightness { value } => apply_vcp(display, VcpCode::Brightness, *value),
        Commands::Contrast { value } => apply_vcp(display, VcpCode::Contrast, *value),
        Commands::Volume { value } => apply_vcp(display, VcpCode::Volume, *value),
        Commands::Power { state } => {
            let p = match state.to_lowercase().as_str() {
                "on" | "1" => PowerState::On,
                "off" | "0" => PowerState::Off,
                _ => {
                    eprintln!("Error: Unrecognized power state '{state}'. Use 'on' or 'off'.");
                    process::exit(1);
                }
            };
            display.inner.set_power(p).map(|()| {
                println!("Power set to {state}");
            })
        }
        Commands::Input { source } => {
            let src = parse_input_source(source.as_str()).map_err(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });
            display.inner.set_input(src.unwrap())
        }
    };

    if let Err(e) = result {
        handle_ddc_error(e);
        std::process::exit(1);
    }
}

/// Parse input source string to InputSource enum.
fn parse_input_source(source: &str) -> Result<InputSource, String> {
    match source.to_lowercase().as_str() {
        "hdmi1" | "hdmi-1" | "hdmi1.0" => Ok(InputSource::Hdmi1),
        "hdmi2" | "hdmi-2" | "hdmi2.0" => Ok(InputSource::Hdmi2),
        "dp1" | "displayport1" | "displayport1.0" => Ok(InputSource::DisplayPort1),
        "dp2" | "displayport2" | "displayport2.0" => Ok(InputSource::DisplayPort2),
        other => Err(format!(
            "Unrecognized input source '{other}'. Use hdmi1, hdmi2, dp1, or dp2."
        )),
    }
}

/// Helper to apply VCP values with verification and basic range clamping.
fn apply_vcp(display: &DisplayDevice, code: VcpCode, value: u32) -> Result<(), DdcError> {
    // Clamp value to 0-100 range and log if clamping occurred
    let val = value.min(100);
    if val != value {
        eprintln!("Warning: value clamped from {value} to {val}");
    }
    display.inner.set_vcp_feature(code as u8, val).map(|()| {
        println!("Successfully set {:?} to {}", code, val);
    })
}

/// Centralized error handling for DDC operations.
fn handle_ddc_error(err: DdcError) {
    match err {
        DdcError::AccessDenied => {
            eprintln!("Error: Access denied. Check I2C permissions (or run as admin/root).")
        }
        DdcError::CommunicationFailed { reason } => {
            eprintln!("Error: Hardware communication failed: {reason}")
        }
        DdcError::UnsupportedFeature => {
            eprintln!("Error: Feature not supported by this monitor.")
        }
        DdcError::InvalidDevice { path } => {
            eprintln!("Error: Invalid DDC device: {path}")
        }
        DdcError::BackendNotAvailable { details } => {
            eprintln!("Error: DDC backend not available: {details}")
        }
    }
}
