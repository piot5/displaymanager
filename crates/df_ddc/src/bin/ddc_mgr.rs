use clap::{Parser, Subcommand};
use df_ddc::list_monitors;
use df_ddc::ddc_types::{PowerState, VcpCode, InputSource};
use df_ddc::error::DdcError;
use df_ddc::ddc_trait::DisplayDevice;

/// CLI tool to manage DDC/CI capable monitors.
#[derive(Parser)]
#[command(name = "monmgr", version = "1.0", about = "DDC/CI Monitor Manager")]
struct Cli {
    /// Select monitor by index (default: 0)
    #[arg(short, long, default_value_t = 0)]
    id: usize,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all connected DDC-capable monitors
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
    let cli = Cli::parse();
    let monitors = list_monitors();

    if monitors.is_empty() {
        eprintln!("Error: No DDC/CI capable monitors found.");
        return;
    }

    // Select the monitor based on the index provided by the user
    let display = monitors.get(cli.id).unwrap_or_else(|| {
        eprintln!("Error: Monitor index {} not found.", cli.id);
        std::process::exit(1);
    });

    match &cli.command {
        Commands::List => {
            for (i, m) in monitors.iter().enumerate() {
                println!("[{}] {}", i, m.info);
            }
        }
        Commands::Brightness { value } => {
            apply_vcp(display, VcpCode::Brightness, *value);
        }
        Commands::Contrast { value } => {
            apply_vcp(display, VcpCode::Contrast, *value);
        }
        Commands::Volume { value } => {
            apply_vcp(display, VcpCode::Volume, *value);
        }
        Commands::Power { state } => {
            let p = if state == "off" { PowerState::Off } else { PowerState::On };
            if let Err(e) = display.inner.set_power(p) {
                handle_ddc_error(e);
            } else {
                println!("Power set to {}", state);
            }
        }
        Commands::Input { source } => {
            let src = match source.as_str() {
                "hdmi1" => InputSource::Hdmi1,
                "hdmi2" => InputSource::Hdmi2,
                "dp1"   => InputSource::DisplayPort1,
                "dp2"   => InputSource::DisplayPort2,
                _       => { eprintln!("Error: Unknown input source."); return; }
            };
            if let Err(e) = display.inner.set_input(src) {
                handle_ddc_error(e);
            } else {
                println!("Input switched to {}", source);
            }
        }
    }
}

/// Helper to apply VCP values with verification and basic range clamping.
fn apply_vcp(display: &DisplayDevice, code: VcpCode, value: u32) {
    let val = value.min(100); // Ensure the value is within the 0-100 range
    if let Err(e) = display.inner.set_vcp_feature(code as u8, val) {
        handle_ddc_error(e);
    } else {
        println!("Successfully set {:?} to {}", code, val);
    }
}

/// Centralized error handling for DDC operations.
fn handle_ddc_error(err: DdcError) {
    match err {
        DdcError::AccessDenied => eprintln!("Error: Access denied. Check I2C permissions (or run as admin/root)."),
        DdcError::CommunicationFailed { reason } => eprintln!("Error: Hardware communication failed: {}", reason),
        DdcError::UnsupportedFeature => eprintln!("Error: Feature not supported by this monitor."),
        _ => eprintln!("An unexpected error occurred: {:?}", err),
    }
}